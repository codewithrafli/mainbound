import { invoke } from '@tauri-apps/api/core'

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

  async function selectFile(repo: string, file: FileChange) {
    selected.value = { repo, file }
    diffLoading.value = true
    try {
      diff.value = await invoke<string>('git_diff', {
        repo,
        path: file.path,
        staged: file.staged,
        untracked: file.status === 'U'
      })
    } catch (e) {
      diff.value = ''
      error.value = String(e)
    } finally {
      diffLoading.value = false
    }
  }

  async function stage(repo: string, paths: string[]) {
    await invoke('git_stage', { repo, paths })
    await refresh(repo)
  }

  async function stageAll(repo: string) {
    await invoke('git_stage_all', { repo })
    await refresh(repo)
  }

  async function unstage(repo: string, paths: string[]) {
    await invoke('git_unstage', { repo, paths })
    await refresh(repo)
  }

  /** Destructive: caller must have confirmed with the user already. */
  async function discard(repo: string, file: FileChange) {
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

  return {
    statusByRepo, logByRepo, selectedRepo, selected, diff, diffLoading,
    committing, error, status, log,
    changeCount, refresh, refreshAll, selectRepo, selectFile,
    stage, stageAll, unstage, discard, commit
  }
})
