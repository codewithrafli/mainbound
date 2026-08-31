import { invoke } from '@tauri-apps/api/core'

export interface ExplorerEntry {
  name: string
  path: string
  kind: 'dir' | 'file'
  size: number | null
  modified: number | null
}

export interface ExplorerFile {
  path: string
  content: string
  size: number
  truncated: boolean
}

export const useExplorerStore = defineStore('explorer', () => {
  const workspaces = useWorkspacesStore()

  const childrenByPath = ref<Record<string, ExplorerEntry[]>>({})
  const expandedByPath = ref<Record<string, boolean>>({ '': true })
  const loadingDirs = ref<Record<string, boolean>>({})
  const selectedPath = ref<string | null>(null)
  const currentFile = ref<ExplorerFile | null>(null)
  const loadingFile = ref(false)
  const error = ref<string | null>(null)

  const rootPath = computed(() => workspaces.active?.path ?? null)
  const rootName = computed(() => workspaces.active?.name ?? 'Workspace')
  const rootEntries = computed(() => childrenByPath.value[''] ?? [])

  function reset() {
    childrenByPath.value = {}
    expandedByPath.value = { '': true }
    loadingDirs.value = {}
    selectedPath.value = null
    currentFile.value = null
    loadingFile.value = false
    error.value = null
  }

  function isExpanded(path: string) {
    return expandedByPath.value[path] ?? false
  }

  function isLoading(path: string) {
    return loadingDirs.value[path] ?? false
  }

  async function loadDir(path = '', force = false) {
    const root = rootPath.value
    if (!root) return
    if (!force && childrenByPath.value[path]) return

    loadingDirs.value = { ...loadingDirs.value, [path]: true }
    try {
      const entries = await invoke<ExplorerEntry[]>('explorer_list_dir', {
        root,
        path: path || null
      })
      childrenByPath.value = { ...childrenByPath.value, [path]: entries }
      error.value = null
    } catch (e) {
      error.value = String(e)
    } finally {
      loadingDirs.value = { ...loadingDirs.value, [path]: false }
    }
  }

  async function ensureRootLoaded() {
    if (rootPath.value && !childrenByPath.value['']) {
      await loadDir('')
    }
  }

  async function refresh() {
    const root = rootPath.value
    reset()
    if (!root) return
    await loadDir('', true)
  }

  async function toggleDir(entry: ExplorerEntry) {
    const next = !isExpanded(entry.path)
    expandedByPath.value = { ...expandedByPath.value, [entry.path]: next }
    selectedPath.value = entry.path
    currentFile.value = null
    if (next) await loadDir(entry.path)
  }

  async function selectFile(entry: ExplorerEntry) {
    const root = rootPath.value
    if (!root || entry.kind !== 'file') return

    selectedPath.value = entry.path
    currentFile.value = null
    loadingFile.value = true
    try {
      currentFile.value = await invoke<ExplorerFile>('explorer_read_file', {
        root,
        path: entry.path
      })
      error.value = null
    } catch (e) {
      error.value = String(e)
    } finally {
      loadingFile.value = false
    }
  }

  watch(rootPath, reset)

  return {
    childrenByPath,
    expandedByPath,
    loadingDirs,
    selectedPath,
    currentFile,
    loadingFile,
    error,
    rootPath,
    rootName,
    rootEntries,
    reset,
    isExpanded,
    isLoading,
    loadDir,
    ensureRootLoaded,
    refresh,
    toggleDir,
    selectFile
  }
})
