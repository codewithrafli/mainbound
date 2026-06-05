export interface TerminalSession {
  id: string
  cwd: string | null
  title: string
}

export const useTerminalsStore = defineStore('terminals', () => {
  const sessions = ref<TerminalSession[]>([])
  const activeId = ref<string | null>(null)

  const active = computed(() =>
    sessions.value.find(s => s.id === activeId.value) ?? null
  )

  /**
   * Registers a new session and makes it active. The PTY itself is
   * spawned by TerminalPane on mount, after its event listeners are
   * attached (avoids losing the first prompt bytes).
   */
  function create(cwd: string | null = null): TerminalSession {
    const session: TerminalSession = {
      id: crypto.randomUUID(),
      cwd,
      title: 'zsh'
    }
    sessions.value.push(session)
    activeId.value = session.id
    return session
  }

  function remove(id: string) {
    const idx = sessions.value.findIndex(s => s.id === id)
    if (idx === -1) return
    sessions.value.splice(idx, 1)
    if (activeId.value === id) {
      activeId.value = sessions.value[Math.min(idx, sessions.value.length - 1)]?.id ?? null
    }
  }

  async function kill(id: string) {
    const { invoke } = await import('@tauri-apps/api/core')
    await invoke('pty_kill', { id }).catch(() => {})
    remove(id)
  }

  function setActive(id: string) {
    activeId.value = id
  }

  return { sessions, activeId, active, create, remove, kill, setActive }
})
