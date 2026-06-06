import { invoke } from '@tauri-apps/api/core'

export interface GhUser {
  login: string
  name: string | null
  avatar_url: string | null
}

export interface RemoteInfo {
  owner: string
  name: string
}

export interface Pr {
  number: number
  title: string
  state: string
  draft: boolean
  head_ref: string
  base_ref: string
  head_sha: string
  html_url: string
  author: string
  created_at: string
}

export interface CheckSummary {
  total: number
  passed: number
  failed: number
  pending: number
}

export interface GhStatusResponse {
  user: GhUser | null
  accounts: string[]
  active: string | null
}

export interface TimelineItem {
  kind: 'comment' | 'review' | 'commit' | 'event'
  author: string
  avatar_url: string | null
  association: string | null
  body: string
  created_at: string
  sha: string | null
  review_state: string | null
  event: string | null
}

export interface ThreadComment {
  id: number
  author: string
  avatar_url: string | null
  association: string | null
  body: string
  created_at: string
  diff_hunk: string | null
}

export interface ReviewThread {
  id: string
  resolved: boolean
  outdated: boolean
  path: string
  line: number | null
  comments: ThreadComment[]
}

export interface CheckRun {
  name: string
  status: string
  conclusion: string | null
  url: string | null
}

export interface PrFile {
  filename: string
  status: string
  additions: number
  deletions: number
  patch: string | null
}

export interface PrDetail {
  number: number
  title: string
  body: string
  state: string
  merged: boolean
  mergeable: boolean | null
  draft: boolean
  head_ref: string
  base_ref: string
  head_sha: string
  author: string
  author_avatar: string | null
  html_url: string
  additions: number
  deletions: number
  commits: number
  changed_files: number
  timeline: TimelineItem[]
  threads: ReviewThread[]
  checks: CheckRun[]
}

export const useGithubStore = defineStore('github', () => {
  const user = ref<GhUser | null>(null)
  const accounts = ref<string[]>([])
  const activeAccount = ref<string | null>(null)
  const initialized = ref(false)
  const remoteByRepo = ref<Record<string, RemoteInfo | null>>({})
  const prsByRepo = ref<Record<string, Pr[]>>({})
  const checksBySha = ref<Record<string, CheckSummary>>({})
  const syncing = ref<'push' | 'pull' | null>(null)
  const loadingPrs = ref(false)
  const error = ref<string | null>(null)

  /** Re-reads auth state and drops per-account caches. */
  async function reload() {
    const status = await invoke<GhStatusResponse>('gh_status').catch(() => null)
    user.value = status?.user ?? null
    accounts.value = status?.accounts ?? []
    activeAccount.value = status?.active ?? null
    prsByRepo.value = {}
    checksBySha.value = {}
  }

  async function init() {
    if (initialized.value) return
    initialized.value = true
    await reload()
  }

  async function connectPat(token: string) {
    await invoke<GhUser>('gh_set_pat', { token })
    await reload()
  }

  async function switchAccount(login: string) {
    if (login === activeAccount.value) return
    await invoke('gh_switch', { login })
    await reload()
  }

  async function logout(login?: string) {
    await invoke('gh_logout', { login: login ?? null })
    await reload()
  }

  async function remoteInfo(repo: string): Promise<RemoteInfo | null> {
    if (!(repo in remoteByRepo.value)) {
      remoteByRepo.value[repo] = await invoke<RemoteInfo | null>('gh_remote_info', { repo }).catch(() => null)
    }
    return remoteByRepo.value[repo] ?? null
  }

  async function listPrs(repo: string) {
    const remote = await remoteInfo(repo)
    if (!remote || !user.value) return
    loadingPrs.value = true
    try {
      const prs = await invoke<Pr[]>('gh_list_prs', { owner: remote.owner, name: remote.name })
      prsByRepo.value[repo] = prs
      error.value = null
      // CI status for the first few PRs (avoid hammering the API)
      for (const pr of prs.slice(0, 10)) {
        invoke<CheckSummary>('gh_pr_checks', {
          owner: remote.owner,
          name: remote.name,
          sha: pr.head_sha
        })
          .then((summary) => {
            checksBySha.value[pr.head_sha] = summary
          })
          .catch(() => {})
      }
    } catch (e) {
      error.value = String(e)
    } finally {
      loadingPrs.value = false
    }
  }

  async function createPr(repo: string, head: string, base: string, title: string, body: string): Promise<Pr | null> {
    const remote = await remoteInfo(repo)
    if (!remote) {
      error.value = 'No GitHub remote found for this repository'
      return null
    }
    try {
      const pr = await invoke<Pr>('gh_create_pr', {
        owner: remote.owner,
        name: remote.name,
        head,
        base,
        title,
        body: body || null
      })
      error.value = null
      await listPrs(repo)
      return pr
    } catch (e) {
      error.value = String(e)
      return null
    }
  }

  // --- in-app PR detail (conversation + checks) ---

  const prDetail = ref<PrDetail | null>(null)
  const prDetailRepo = ref<string | null>(null)
  const loadingDetail = ref(false)
  const prFiles = ref<PrFile[] | null>(null)
  const loadingFiles = ref(false)

  /** Lazy: only fetched when the Files tab is opened. */
  async function loadPrFiles() {
    if (!prDetail.value || !prDetailRepo.value || loadingFiles.value) return
    const remote = await remoteInfo(prDetailRepo.value)
    if (!remote) return
    loadingFiles.value = true
    try {
      prFiles.value = await invoke<PrFile[]>('gh_pr_files', {
        owner: remote.owner,
        name: remote.name,
        number: prDetail.value.number
      })
    } catch (e) {
      error.value = String(e)
    } finally {
      loadingFiles.value = false
    }
  }

  async function openPrDetail(repo: string, number: number) {
    const remote = await remoteInfo(repo)
    if (!remote) return
    prDetailRepo.value = repo
    loadingDetail.value = true
    try {
      prDetail.value = await invoke<PrDetail>('gh_pr_detail', {
        owner: remote.owner,
        name: remote.name,
        number
      })
      error.value = null
    } catch (e) {
      error.value = String(e)
    } finally {
      loadingDetail.value = false
    }
  }

  function closePrDetail() {
    prDetail.value = null
    prDetailRepo.value = null
    prFiles.value = null
  }

  async function refreshPrDetail() {
    if (prDetail.value && prDetailRepo.value) {
      await openPrDetail(prDetailRepo.value, prDetail.value.number)
    }
  }

  async function commentOnPr(body: string): Promise<boolean> {
    if (!prDetail.value || !prDetailRepo.value) return false
    const remote = await remoteInfo(prDetailRepo.value)
    if (!remote) return false
    try {
      await invoke('gh_pr_comment', {
        owner: remote.owner,
        name: remote.name,
        number: prDetail.value.number,
        body
      })
      await refreshPrDetail()
      return true
    } catch (e) {
      error.value = String(e)
      return false
    }
  }

  async function replyToThread(commentId: number, body: string): Promise<boolean> {
    if (!prDetail.value || !prDetailRepo.value) return false
    const remote = await remoteInfo(prDetailRepo.value)
    if (!remote) return false
    try {
      await invoke('gh_pr_reply_thread', {
        owner: remote.owner,
        name: remote.name,
        number: prDetail.value.number,
        commentId,
        body
      })
      await refreshPrDetail()
      return true
    } catch (e) {
      error.value = String(e)
      return false
    }
  }

  async function resolveThread(threadId: string, resolved: boolean): Promise<boolean> {
    try {
      await invoke('gh_pr_resolve_thread', { threadId, resolved })
      await refreshPrDetail()
      return true
    } catch (e) {
      error.value = String(e)
      return false
    }
  }

  async function mergePr(method: 'merge' | 'squash' | 'rebase'): Promise<boolean> {
    if (!prDetail.value || !prDetailRepo.value) return false
    const remote = await remoteInfo(prDetailRepo.value)
    if (!remote) return false
    try {
      await invoke('gh_pr_merge', {
        owner: remote.owner,
        name: remote.name,
        number: prDetail.value.number,
        method
      })
      await refreshPrDetail()
      return true
    } catch (e) {
      error.value = String(e)
      return false
    }
  }

  async function push(repo: string) {
    syncing.value = 'push'
    try {
      await invoke('gh_push', { repo })
      error.value = null
      return true
    } catch (e) {
      error.value = String(e)
      return false
    } finally {
      syncing.value = null
    }
  }

  async function pull(repo: string) {
    syncing.value = 'pull'
    try {
      await invoke('gh_pull', { repo })
      error.value = null
      return true
    } catch (e) {
      error.value = String(e)
      return false
    } finally {
      syncing.value = null
    }
  }

  return {
    user, accounts, activeAccount,
    remoteByRepo, prsByRepo, checksBySha, syncing, loadingPrs, error,
    prDetail, prDetailRepo, loadingDetail, prFiles, loadingFiles, loadPrFiles,
    init, reload, connectPat, switchAccount, logout,
    remoteInfo, listPrs, createPr, push, pull,
    openPrDetail, closePrDetail, refreshPrDetail, commentOnPr,
    replyToThread, resolveThread, mergePr
  }
})
