import { leavesOf } from '~/stores/terminals'

/** Minimum sustained output before "command finished" fires. */
const MIN_BURST_MS = 10_000
/** Silence that marks the end of a burst. */
const QUIET_MS = 2_500
/** Per-session cooldown between notifications. */
const COOLDOWN_MS = 15_000

interface Activity {
  lastAt: number
  burstStart: number | null
  notifiedAt: number
}

export const useNotificationsStore = defineStore('notifications', () => {
  const terminals = useTerminalsStore()

  /** unread badge per tab, cleared when the tab is activated */
  const unreadByTab = ref<Record<string, number>>({})
  const granted = ref(false)

  const activity = new Map<string, Activity>()
  let timer: ReturnType<typeof setInterval> | undefined

  async function init() {
    try {
      const { isPermissionGranted, requestPermission }
        = await import('@tauri-apps/plugin-notification')
      granted.value = await isPermissionGranted()
      if (!granted.value) {
        granted.value = (await requestPermission()) === 'granted'
      }
    } catch {
      granted.value = false
    }
    // burst watcher
    timer ??= setInterval(checkBursts, 1_000)
  }

  function tabOf(sessionId: string) {
    return terminals.tabs.find(t =>
      leavesOf(t.root).some(l => l.sessionId === sessionId)
    ) ?? null
  }

  /** Suppress notifications for what the user is already looking at. */
  function isWatched(sessionId: string): boolean {
    if (!document.hasFocus()) return false
    const tab = tabOf(sessionId)
    return !!tab && tab.id === terminals.activeTabId
  }

  function entry(sessionId: string): Activity {
    let a = activity.get(sessionId)
    if (!a) {
      a = { lastAt: 0, burstStart: null, notifiedAt: 0 }
      activity.set(sessionId, a)
    }
    return a
  }

  async function fire(sessionId: string, title: string, body: string) {
    const a = entry(sessionId)
    const now = Date.now()
    if (now - a.notifiedAt < COOLDOWN_MS) return
    a.notifiedAt = now

    const tab = tabOf(sessionId)
    if (tab) {
      unreadByTab.value[tab.id] = (unreadByTab.value[tab.id] ?? 0) + 1
    }
    // dispatch via Rust: notification plugin with osascript fallback —
    // never gated on a cached permission snapshot
    const { invoke } = await import('@tauri-apps/api/core')
    invoke('notify', { title, body }).catch(() => {})
  }

  function sessionLabel(sessionId: string): string {
    const tab = tabOf(sessionId)
    const session = terminals.sessions[sessionId]
    return tab?.title ?? session?.title ?? 'session'
  }

  /** BEL — CLIs (incl. Claude Code) ring it when they need attention. */
  function onBell(sessionId: string) {
    if (isWatched(sessionId)) return
    fire(sessionId, 'Needs attention', `Session "${sessionLabel(sessionId)}" rang the bell`)
  }

  /** OSC 9 / OSC 777 — explicit terminal notifications. */
  function onOscNotify(sessionId: string, title: string, body: string) {
    if (isWatched(sessionId)) return
    fire(sessionId, title || `tide — ${sessionLabel(sessionId)}`, body)
  }

  /** Every output chunk feeds the burst heuristic. */
  function onOutput(sessionId: string) {
    const now = Date.now()
    const a = entry(sessionId)
    if (a.burstStart === null || now - a.lastAt > QUIET_MS) {
      a.burstStart = now
    }
    a.lastAt = now
  }

  /** Long output burst followed by silence = a command finished. */
  function checkBursts() {
    const now = Date.now()
    for (const [sessionId, a] of activity) {
      if (a.burstStart === null) continue
      if (now - a.lastAt < QUIET_MS) continue
      const duration = a.lastAt - a.burstStart
      a.burstStart = null
      if (duration >= MIN_BURST_MS && !isWatched(sessionId)) {
        const secs = Math.round(duration / 1000)
        fire(sessionId, 'Command finished', `"${sessionLabel(sessionId)}" went quiet after ${secs}s of output`)
      }
    }
  }

  function forget(sessionId: string) {
    activity.delete(sessionId)
  }

  // activating a tab clears its badge
  watch(() => terminals.activeTabId, (id) => {
    if (id && unreadByTab.value[id]) {
      unreadByTab.value = { ...unreadByTab.value, [id]: 0 }
    }
  })

  return { unreadByTab, granted, init, onBell, onOscNotify, onOutput, forget }
})
