import { invoke } from '@tauri-apps/api/core'

export interface TerminalSession {
  id: string
  cwd: string | null
  title: string
}

export interface PaneLeaf {
  type: 'leaf'
  sessionId: string
}

export interface PaneSplit {
  type: 'split'
  id: string
  direction: 'row' | 'column'
  children: [PaneNode, PaneNode]
  /** percentage sizes of the two children, summing to 100 */
  sizes: [number, number]
}

export type PaneNode = PaneLeaf | PaneSplit

export interface TerminalTab {
  id: string
  title: string
  branch: string | null
  root: PaneNode
}

function basename(path: string | null): string | null {
  if (!path) return null
  return path.split('/').filter(Boolean).pop() ?? null
}

export function leavesOf(node: PaneNode): PaneLeaf[] {
  if (node.type === 'leaf') return [node]
  return node.children.flatMap(leavesOf)
}

/** Replaces the leaf for `sessionId` with a split of it and `subtree`. */
function insertAtLeaf(
  node: PaneNode,
  sessionId: string,
  subtree: PaneNode,
  direction: 'row' | 'column',
  before = false
): PaneNode {
  if (node.type === 'leaf') {
    if (node.sessionId !== sessionId) return node
    return {
      type: 'split',
      id: crypto.randomUUID(),
      direction,
      children: before ? [subtree, node] : [node, subtree],
      sizes: [50, 50]
    }
  }
  node.children = [
    insertAtLeaf(node.children[0], sessionId, subtree, direction, before),
    insertAtLeaf(node.children[1], sessionId, subtree, direction, before)
  ]
  return node
}

/** Removes a leaf; a split with one remaining child collapses to that child. */
function removeLeaf(node: PaneNode, sessionId: string): PaneNode | null {
  if (node.type === 'leaf') {
    return node.sessionId === sessionId ? null : node
  }
  const a = removeLeaf(node.children[0], sessionId)
  const b = removeLeaf(node.children[1], sessionId)
  if (a && b) {
    node.children = [a, b]
    return node
  }
  return a ?? b
}

export const useTerminalsStore = defineStore('terminals', () => {
  const sessions = ref<Record<string, TerminalSession>>({})
  const tabs = ref<TerminalTab[]>([])
  const activeTabId = ref<string | null>(null)
  const focusedByTab = ref<Record<string, string>>({})
  /** tab currently being dragged from the sidebar (for dock-on-drop) */
  const draggingTabId = ref<string | null>(null)
  /** pane currently being dragged by its grab handle */
  const draggingSessionId = ref<string | null>(null)

  const activeTab = computed(() =>
    tabs.value.find(t => t.id === activeTabId.value) ?? null
  )

  const focusedSessionId = computed(() =>
    activeTab.value ? focusedByTab.value[activeTab.value.id] ?? null : null
  )

  const paneCount = computed(() =>
    Object.keys(sessions.value).length
  )

  function newSession(cwd: string | null, title?: string): TerminalSession {
    const session: TerminalSession = {
      id: crypto.randomUUID(),
      cwd,
      title: title ?? basename(cwd) ?? 'zsh'
    }
    sessions.value[session.id] = session
    return session
  }

  /**
   * Opens a new tab with a single pane. The PTY itself is spawned by
   * TerminalPane on mount, after its event listeners are attached
   * (avoids losing the first prompt bytes).
   */
  function create(cwd: string | null = null, title?: string): TerminalTab {
    const session = newSession(cwd, title)
    const tab: TerminalTab = {
      id: crypto.randomUUID(),
      title: title ?? basename(cwd) ?? 'zsh',
      branch: null,
      root: { type: 'leaf', sessionId: session.id }
    }
    tabs.value.push(tab)
    activeTabId.value = tab.id
    focusedByTab.value[tab.id] = session.id
    // Resolve the git branch for the sidebar (async, best-effort)
    if (cwd) {
      const reactiveTab = tabs.value[tabs.value.length - 1]!
      invoke<string | null>('git_branch', { path: cwd })
        .then((branch) => {
          reactiveTab.branch = branch
        })
        .catch(() => {})
    }
    return tab
  }

  /** Splits the focused pane of the active tab; the new pane inherits its cwd. */
  function split(direction: 'row' | 'column') {
    const tab = activeTab.value
    const focused = focusedSessionId.value
    if (!tab || !focused) return
    const origin = sessions.value[focused]
    const session = newSession(origin?.cwd ?? null, origin?.title)
    tab.root = insertAtLeaf(tab.root, focused, {
      type: 'leaf',
      sessionId: session.id
    }, direction)
    focusedByTab.value[tab.id] = session.id
  }

  /**
   * Docks the whole pane tree of `sourceTabId` next to `targetSessionId`
   * (drag & drop): the target leaf becomes a split of [target, source]
   * on the chosen edge, and the source tab disappears.
   */
  function moveTabIntoPane(
    sourceTabId: string,
    targetSessionId: string,
    zone: 'left' | 'right' | 'top' | 'bottom'
  ) {
    const source = tabs.value.find(t => t.id === sourceTabId)
    const target = tabOf(targetSessionId)
    if (!source || !target || target.id === sourceTabId) return
    const direction = zone === 'left' || zone === 'right' ? 'row' : 'column'
    const before = zone === 'left' || zone === 'top'
    target.root = insertAtLeaf(target.root, targetSessionId, source.root, direction, before)
    const idx = tabs.value.findIndex(t => t.id === sourceTabId)
    tabs.value.splice(idx, 1)
    focusedByTab.value = dropKeys(focusedByTab.value, [sourceTabId])
    activeTabId.value = target.id
    focusedByTab.value[target.id] = leavesOf(source.root)[0]!.sessionId
  }

  function tabOf(sessionId: string): TerminalTab | null {
    return tabs.value.find(t =>
      leavesOf(t.root).some(l => l.sessionId === sessionId)
    ) ?? null
  }

  function dropKeys<T>(obj: Record<string, T>, keys: string[]): Record<string, T> {
    return Object.fromEntries(Object.entries(obj).filter(([k]) => !keys.includes(k)))
  }

  /**
   * Re-docks a single pane next to `targetSessionId` (drag & drop within
   * or across tabs): the pane is unhooked from its tree and the target
   * leaf becomes a split of [target, pane] on the chosen edge.
   */
  function movePaneIntoPane(
    sourceSessionId: string,
    targetSessionId: string,
    zone: 'left' | 'right' | 'top' | 'bottom'
  ) {
    if (sourceSessionId === targetSessionId) return
    const sourceTab = tabOf(sourceSessionId)
    const targetTab = tabOf(targetSessionId)
    if (!sourceTab || !targetTab) return

    const remaining = removeLeaf(sourceTab.root, sourceSessionId)
    if (remaining) {
      sourceTab.root = remaining
      if (focusedByTab.value[sourceTab.id] === sourceSessionId) {
        focusedByTab.value[sourceTab.id] = leavesOf(remaining)[0]!.sessionId
      }
    } else {
      // source tab is now empty — it can only be a different tab,
      // because same-tab moves always leave the target leaf behind
      const idx = tabs.value.findIndex(t => t.id === sourceTab.id)
      tabs.value.splice(idx, 1)
      focusedByTab.value = dropKeys(focusedByTab.value, [sourceTab.id])
    }

    const direction = zone === 'left' || zone === 'right' ? 'row' : 'column'
    const before = zone === 'left' || zone === 'top'
    targetTab.root = insertAtLeaf(targetTab.root, targetSessionId, {
      type: 'leaf',
      sessionId: sourceSessionId
    }, direction, before)
    activeTabId.value = targetTab.id
    focusedByTab.value[targetTab.id] = sourceSessionId
  }

  /** Tree surgery after a pane's PTY is gone (exited or killed). */
  function closePane(sessionId: string) {
    sessions.value = dropKeys(sessions.value, [sessionId])
    const tab = tabOf(sessionId)
    if (!tab) return
    const root = removeLeaf(tab.root, sessionId)
    if (!root) {
      const idx = tabs.value.findIndex(t => t.id === tab.id)
      tabs.value.splice(idx, 1)
      focusedByTab.value = dropKeys(focusedByTab.value, [tab.id])
      if (activeTabId.value === tab.id) {
        activeTabId.value = tabs.value[Math.min(idx, tabs.value.length - 1)]?.id ?? null
      }
      return
    }
    tab.root = root
    if (focusedByTab.value[tab.id] === sessionId) {
      focusedByTab.value[tab.id] = leavesOf(root)[0]!.sessionId
    }
  }

  async function kill(sessionId: string) {
    await invoke('pty_kill', { id: sessionId }).catch(() => {})
    closePane(sessionId)
  }

  async function killTab(tabId: string) {
    const tab = tabs.value.find(t => t.id === tabId)
    if (!tab) return
    const sessionIds = leavesOf(tab.root).map(l => l.sessionId)
    for (const id of sessionIds) {
      await invoke('pty_kill', { id }).catch(() => {})
    }
    sessions.value = dropKeys(sessions.value, sessionIds)
    const idx = tabs.value.findIndex(t => t.id === tabId)
    tabs.value.splice(idx, 1)
    focusedByTab.value = dropKeys(focusedByTab.value, [tabId])
    if (activeTabId.value === tabId) {
      activeTabId.value = tabs.value[Math.min(idx, tabs.value.length - 1)]?.id ?? null
    }
  }

  function setActiveTab(id: string) {
    activeTabId.value = id
  }

  function focusPane(sessionId: string) {
    const tab = tabOf(sessionId)
    if (!tab) return
    activeTabId.value = tab.id
    focusedByTab.value[tab.id] = sessionId
  }

  return {
    sessions, tabs, activeTabId, focusedByTab, draggingTabId, draggingSessionId,
    activeTab, focusedSessionId, paneCount,
    create, split, moveTabIntoPane, movePaneIntoPane,
    closePane, kill, killTab, setActiveTab, focusPane
  }
})
