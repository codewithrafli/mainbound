import { invoke } from '@tauri-apps/api/core'
import type { CheckSummary, Pr } from '~/stores/github'

export interface ReviewSummary {
  approved: number
  changes_requested: number
  commented: number
}

/**
 * PR Cockpit: always-visible state for the ACTIVE terminal session's
 * repository — branch, local changes, ahead/behind, the branch's open
 * PR, its review state and CI checks.
 */
export const useCockpitStore = defineStore('cockpit', () => {
  const terminals = useTerminalsStore()
  const git = useGitStore()
  const github = useGithubStore()

  const repoByCwd = ref<Record<string, string | null>>({})
  const prByRepo = ref<Record<string, Pr | null>>({})
  const checksByRepo = ref<Record<string, CheckSummary | null>>({})
  const reviewsByRepo = ref<Record<string, ReviewSummary | null>>({})
  const refreshing = ref(false)

  const activeCwd = computed(() => {
    const sid = terminals.focusedSessionId
    return sid ? terminals.sessions[sid]?.cwd ?? null : null
  })

  const activeRepo = computed(() =>
    activeCwd.value ? repoByCwd.value[activeCwd.value] ?? null : null
  )

  const status = computed(() =>
    activeRepo.value ? git.statusByRepo[activeRepo.value] ?? null : null
  )
  const pr = computed(() =>
    activeRepo.value ? prByRepo.value[activeRepo.value] ?? null : null
  )
  const checks = computed(() =>
    activeRepo.value ? checksByRepo.value[activeRepo.value] ?? null : null
  )
  const reviews = computed(() =>
    activeRepo.value ? reviewsByRepo.value[activeRepo.value] ?? null : null
  )
  const changeCount = computed(() =>
    status.value
      ? new Set([...status.value.staged, ...status.value.unstaged].map(f => f.path)).size
      : 0
  )

  async function resolveRepo(cwd: string): Promise<string | null> {
    if (!(cwd in repoByCwd.value)) {
      repoByCwd.value[cwd] = await invoke<string | null>('git_repo_root', { path: cwd })
        .catch(() => null)
    }
    return repoByCwd.value[cwd] ?? null
  }

  async function refreshGh(repo: string) {
    if (!github.user) return
    const remote = await github.remoteInfo(repo)
    const branch = git.statusByRepo[repo]?.branch
    if (!remote || !branch) return
    const found = await invoke<Pr | null>('gh_pr_for_branch', {
      owner: remote.owner,
      name: remote.name,
      branch
    }).catch(() => null)
    prByRepo.value[repo] = found
    if (found) {
      invoke<CheckSummary>('gh_pr_checks', {
        owner: remote.owner,
        name: remote.name,
        sha: found.head_sha
      })
        .then((c) => {
          checksByRepo.value[repo] = c
        })
        .catch(() => {})
      invoke<ReviewSummary>('gh_pr_reviews', {
        owner: remote.owner,
        name: remote.name,
        number: found.number
      })
        .then((r) => {
          reviewsByRepo.value[repo] = r
        })
        .catch(() => {})
    } else {
      checksByRepo.value[repo] = null
      reviewsByRepo.value[repo] = null
    }
  }

  async function refresh() {
    const cwd = activeCwd.value
    if (!cwd) return
    const repo = await resolveRepo(cwd)
    if (!repo) return
    refreshing.value = true
    try {
      await git.refresh(repo)
      await refreshGh(repo)
    } finally {
      refreshing.value = false
    }
  }

  // Follow the focused session
  watch(activeCwd, (cwd) => {
    if (cwd) refresh()
  }, { immediate: true })

  return {
    repoByCwd, prByRepo, checksByRepo, reviewsByRepo, refreshing,
    activeCwd, activeRepo, status, pr, checks, reviews, changeCount,
    resolveRepo, refresh, refreshGh
  }
})
