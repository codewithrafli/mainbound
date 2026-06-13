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

  async function reorder(ids: string[]) {
    const byId = new Map(list.value.map(workspace => [workspace.id, workspace]))
    const next = ids
      .map(id => byId.get(id))
      .filter((workspace): workspace is Workspace => !!workspace)
    const seen = new Set(next.map(workspace => workspace.id))
    next.push(...list.value.filter(workspace => !seen.has(workspace.id)))
    list.value = next
    await invoke('workspace_reorder', { ids: next.map(workspace => workspace.id) })
  }

  async function move(id: string, direction: -1 | 1) {
    const index = list.value.findIndex(w => w.id === id)
    const target = index + direction
    if (index < 0 || target < 0 || target >= list.value.length) return
    const next = [...list.value]
    const current = next[index]
    const other = next[target]
    if (!current || !other) return
    next[index] = other
    next[target] = current
    await reorder(next.map(w => w.id))
  }

  async function remove(id: string) {
    await invoke('workspace_remove', { id })
    list.value = list.value.filter(w => w.id !== id)
    if (activeId.value === id) {
      activeId.value = list.value[0]?.id ?? null
      await scan()
    }
  }

  /** Called by git store to check if a repo path still belongs to this workspace. */
  function hasRepo(repoPath: string): boolean {
    return repos.value.some(r => r.path === repoPath)
  }

  return { list, activeId, active, repos, scanning, initialized, init, add, setActive, reorder, move, remove, scan, hasRepo }
})
