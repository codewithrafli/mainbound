import { invoke } from '@tauri-apps/api/core'

export interface Workspace {
  id: string
  name: string
  path: string
}

export interface RepoInfo {
  path: string
  name: string
  branch: string | null
}

export const useWorkspacesStore = defineStore('workspaces', () => {
  const list = ref<Workspace[]>([])
  const activeId = ref<string | null>(null)
  const repos = ref<RepoInfo[]>([])
  const scanning = ref(false)
  const initialized = ref(false)

  const active = computed(() =>
    list.value.find(w => w.id === activeId.value) ?? null
  )

  async function scan() {
    if (!active.value) {
      repos.value = []
      return
    }
    scanning.value = true
    try {
      repos.value = await invoke<RepoInfo[]>('repo_scan', { path: active.value.path })
    } finally {
      scanning.value = false
    }
  }

  async function init() {
    if (initialized.value) return
    initialized.value = true
    const [workspaces, last] = await invoke<[Workspace[], string | null]>('workspace_list')
    list.value = workspaces
    activeId.value = last ?? workspaces[0]?.id ?? null
    await scan()
  }

  async function add() {
    const { open } = await import('@tauri-apps/plugin-dialog')
    const path = await open({ directory: true, multiple: false, title: 'Add Workspace' })
    if (!path) return null
    const workspace = await invoke<Workspace>('workspace_add', { path })
    if (!list.value.some(w => w.id === workspace.id)) {
      list.value.push(workspace)
    }
    activeId.value = workspace.id
    await scan()
    return workspace
  }

  async function setActive(id: string) {
    if (activeId.value === id) return
    activeId.value = id
    await invoke('workspace_set_last', { id }).catch(() => {})
    await scan()
  }

  async function remove(id: string) {
    await invoke('workspace_remove', { id })
    list.value = list.value.filter(w => w.id !== id)
    if (activeId.value === id) {
      activeId.value = list.value[0]?.id ?? null
      await scan()
    }
  }

  return { list, activeId, active, repos, scanning, init, add, setActive, remove, scan }
})
