import { invoke } from '@tauri-apps/api/core'

export interface StashEntry {
  index: number
  message: string
  branch: string
  date: string
}

export interface BlameLine {
  line: number
  hash: string
  short_hash: string
  author: string
  timestamp: number
  summary: string
}

export interface FileChange {
  path: string
  orig_path: string | null
  status: string // M, A, D, R, C, U (untracked), ! (conflict)
  staged: boolean
  added: number | null
  removed: number | null
}

export interface GitStatus {
  branch: string | null
  oid: string | null
  ahead: number
  behind: number
  staged: FileChange[]
  unstaged: FileChange[]
  conflicts: string[]
}

export interface Commit {
  hash: string
  short_hash: string
  author: string
  date: string
  subject: string
}

export interface SelectedFile {
  repo: string
  file: FileChange
}

export const useGitStore = defineStore('git', () => {
  const statusByRepo = ref<Record<string, GitStatus>>({})
  const logByRepo = ref<Record<string, Commit[]>>({})
  const selectedRepo = ref<string | null>(null)
  const selected = ref<SelectedFile | null>(null)
  const diff = ref('')
  const diffLoading = ref(false)
  const committing = ref(false)
  const error = ref<string | null>(null)
  // prevents concurrent stage/unstage/discard on the same repo
  const gitBusy = ref(false)
  const stashList = ref<StashEntry[]>([])
  const blameByPath = ref<Record<string, BlameLine[]>>({})
  const fileSearch = ref('')
  const prTemplate = ref<string | null>(null)

  const status = computed(() =>
    selectedRepo.value ? statusByRepo.value[selectedRepo.value] ?? null : null
  )
  const log = computed(() =>
    selectedRepo.value ? logByRepo.value[selectedRepo.value] ?? [] : []
  )

  function changeCount(repo: string): number {
    const s = statusByRepo.value[repo]
    if (!s) return 0
    // staged + unstaged, deduped by path
    return new Set([...s.staged, ...s.unstaged].map(f => f.path)).size
  }

  async function refresh(repo: string) {
    try {
      const [s, l] = await Promise.all([
        invoke<GitStatus>('git_status', { repo }),
        invoke<Commit[]>('git_log', { repo, limit: 15 })
      ])
      statusByRepo.value[repo] = s
      logByRepo.value[repo] = l
      error.value = null
    } catch (e) {
      error.value = String(e)
    }
  }

  /** Refresh change counts for the repo list (status only, no log). */
  async function refreshAll(repos: string[]) {
    await Promise.all(repos.map(async (repo) => {
      try {
        statusByRepo.value[repo] = await invoke<GitStatus>('git_status', { repo })
      } catch {
        // not a repo / git failed — leave it out
      }
    }))
  }

  async function selectRepo(repo: string) {
    selectedRepo.value = repo
    selected.value = null
    diff.value = ''
    await refresh(repo)
  }

  let diffSeq = 0
  async function selectFile(repo: string, file: FileChange) {
    selected.value = { repo, file }
    diff.value = '' // clear immediately so stale diff never shows
    diffLoading.value = true
    const seq = ++diffSeq
    try {
      const result = await invoke<string>('git_diff', {
        repo,
        path: file.path,
        staged: file.staged,
        untracked: file.status === 'U'
      })
      // discard if another selectFile was called while awaiting
      if (seq === diffSeq) diff.value = result
    } catch (e) {
      if (seq === diffSeq) {
        diff.value = ''
        error.value = String(e)
      }
    } finally {
      if (seq === diffSeq) diffLoading.value = false
    }
  }

  async function stage(repo: string, paths: string[]) {
    if (gitBusy.value) return
    gitBusy.value = true
    try {
      await invoke('git_stage', { repo, paths })
      await refresh(repo)
    } finally { gitBusy.value = false }
  }

  async function stageAll(repo: string) {
    if (gitBusy.value) return
    gitBusy.value = true
    try {
      await invoke('git_stage_all', { repo })
      await refresh(repo)
    } finally { gitBusy.value = false }
  }

  async function unstage(repo: string, paths: string[]) {
    if (gitBusy.value) return
    gitBusy.value = true
    try {
      await invoke('git_unstage', { repo, paths })
      await refresh(repo)
    } finally { gitBusy.value = false }
  }

  /** Destructive: caller must have confirmed with the user already. */
  async function discard(repo: string, file: FileChange) {
    if (gitBusy.value) return
    gitBusy.value = true
    try {
      const untracked = file.status === 'U'
      await invoke('git_discard', {
        repo,
        tracked: untracked ? [] : [file.path],
        untracked: untracked ? [file.path] : []
      })
      if (selected.value?.file.path === file.path) {
        selected.value = null
        diff.value = ''
      }
      await refresh(repo)
    } finally { gitBusy.value = false }
  }

  async function commit(repo: string, summary: string, description: string) {
    committing.value = true
    try {
      await invoke('git_commit', { repo, summary, description: description || null })
      await refresh(repo)
      error.value = null
      return true
    } catch (e) {
      error.value = String(e)
      return false
    } finally {
      committing.value = false
    }
  }

  async function amendCommit(repo: string, summary: string, description: string) {
    committing.value = true
    try {
      await invoke('git_commit_amend', { repo, summary, description: description || null })
      await refresh(repo)
      error.value = null
      return true
    } catch (e) {
      error.value = String(e)
      return false
    } finally {
      committing.value = false
    }
  }

  async function cherryPick(repo: string, hash: string) {
    try {
      await invoke('git_cherry_pick', { repo, hash })
      await refresh(repo)
      error.value = null
      return true
    } catch (e) {
      error.value = String(e)
      return false
    }
  }

  async function refreshStash(repo: string) {
    try {
      stashList.value = await invoke<StashEntry[]>('git_stash_list', { repo })
    } catch {
      stashList.value = []
    }
  }

  async function stashPush(repo: string, message?: string) {
    try {
      await invoke('git_stash_push', { repo, message: message || null })
      await Promise.all([refresh(repo), refreshStash(repo)])
      return true
    } catch (e) {
      error.value = String(e)
      return false
    }
  }

  async function stashApply(repo: string, index: number) {
    try {
      await invoke('git_stash_apply', { repo, index })
      await refresh(repo)
      return true
    } catch (e) {
      error.value = String(e)
      return false
    }
  }

  async function stashDrop(repo: string, index: number) {
    try {
      await invoke('git_stash_drop', { repo, index })
      await refreshStash(repo)
      return true
    } catch (e) {
      error.value = String(e)
      return false
    }
  }

  async function loadBlame(repo: string, path: string) {
    // Keep at most 20 blame entries to avoid unbounded memory growth
    const keys = Object.keys(blameByPath.value)
    if (keys.length >= 20) {
      const { [keys[0]!]: _, ...rest } = blameByPath.value
      blameByPath.value = rest
    }
    try {
      blameByPath.value[path] = await invoke<BlameLine[]>('git_blame', { repo, path })
    } catch {
      blameByPath.value[path] = []
    }
  }

  async function stageHunk(repo: string, patch: string) {
    try {
      await invoke('git_stage_hunk', { repo, patch })
      await refresh(repo)
      error.value = null
      return true
    } catch (e) {
      error.value = String(e)
      return false
    }
  }

  async function conflictResolve(repo: string, path: string, resolution: 'ours' | 'theirs') {
    try {
      await invoke('git_conflict_resolve', { repo, path, resolution })
      await refresh(repo)
      error.value = null
      return true
    } catch (e) {
      error.value = String(e)
      return false
    }
  }

  async function loadPrTemplate(repo: string) {
    prTemplate.value = await invoke<string | null>('git_pr_template', { repo }).catch(() => null)
  }

  // ---------------------------------------------------------------------------
  // Ship It — full flow: stage all → AI commit → commit → push
  // ---------------------------------------------------------------------------

  type ShipStep = 'staging' | 'generating' | 'committing' | 'pushing' | 'done'
  const shipStep = ref<ShipStep | null>(null)
  const shipError = ref<string | null>(null)

  async function shipIt(
    repo: string,
    provider: string,
    onPush: () => Promise<void>
  ): Promise<boolean> {
    shipStep.value = 'staging'
    shipError.value = null
    try {
      await invoke('git_stage_all', { repo })
      await refresh(repo)

      shipStep.value = 'generating'
      const suggestion = await invoke<{ summary: string, description: string }>(
        'ai_commit_message', { repo, provider }
      )

      shipStep.value = 'committing'
      await invoke('git_commit', {
        repo,
        summary: suggestion.summary,
        description: suggestion.description || null
      })
      await refresh(repo)

      shipStep.value = 'pushing'
      await onPush()

      shipStep.value = 'done'
      setTimeout(() => {
        shipStep.value = null
      }, 2000)
      return true
    } catch (e) {
      shipError.value = String(e)
      shipStep.value = null
      return false
    }
  }

  // ---------------------------------------------------------------------------
  // Checkpoint — named stash with "checkpoint:" prefix
  // ---------------------------------------------------------------------------

  const checkpoints = computed(() =>
    stashList.value.filter(s => s.message.startsWith('checkpoint:'))
  )

  async function checkpointSave(repo: string, name?: string) {
    const msg = `checkpoint: ${name || new Date().toLocaleTimeString()}`
    return stashPush(repo, msg)
  }

  async function checkpointRestore(repo: string, index: number) {
    return stashApply(repo, index)
  }

  async function checkpointDrop(repo: string, index: number) {
    return stashDrop(repo, index)
  }

  // ---------------------------------------------------------------------------
  // Bulk staging by folder
  // ---------------------------------------------------------------------------

  /** Returns unstaged files grouped by their top-level directory. */
  const unstagedByFolder = computed(() => {
    const s = status.value
    if (!s) return {} as Record<string, FileChange[]>
    const groups: Record<string, FileChange[]> = {}
    for (const file of s.unstaged) {
      const folder = file.path.includes('/')
        ? file.path.split('/')[0]!
        : '.'
      if (!groups[folder]) groups[folder] = []
      groups[folder]!.push(file)
    }
    return groups
  })

  async function stageFolder(repo: string, folder: string) {
    const files = unstagedByFolder.value[folder]
    if (!files?.length) return
    await stage(repo, files.map((file: FileChange) => file.path))
  }

  const filteredUnstaged = computed(() => {
    const s = status.value
    if (!s) return []
    if (!fileSearch.value.trim()) return s.unstaged
    const q = fileSearch.value.toLowerCase()
    return s.unstaged.filter(f => f.path.toLowerCase().includes(q))
  })

  /** Clear the selected repo + diff when switching workspaces. */
  function resetSelection() {
    selectedRepo.value = null
    selected.value = null
    diff.value = ''
    error.value = null
    fileSearch.value = ''
    stashList.value = []
    prTemplate.value = null
  }

  return {
    statusByRepo, logByRepo, selectedRepo, selected, diff, diffLoading,
    committing, gitBusy, error, status, log, stashList, blameByPath, fileSearch,
    prTemplate, filteredUnstaged, unstagedByFolder,
    shipStep, shipError, checkpoints,
    changeCount, refresh, refreshAll, selectRepo, selectFile,
    stage, stageAll, unstage, discard, commit, amendCommit, cherryPick,
    refreshStash, stashPush, stashApply, stashDrop,
    loadBlame, stageHunk, conflictResolve, loadPrTemplate, resetSelection,
    shipIt, checkpointSave, checkpointRestore, checkpointDrop, stageFolder
  }
})
