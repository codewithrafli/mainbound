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
  /** owning workspace; null = not tied to a workspace (e.g. home dir) */
  workspaceId: string | null
}

export type SavedNode
  = | { type: 'leaf', cwd: string | null, title: string, focused?: boolean }
    | { type: 'split', direction: 'row' | 'column', sizes: [number, number], children: [SavedNode, SavedNode] }

export interface SavedTab {
  title: string
  root: SavedNode
  workspaceId?: string | null
}

/** Full restore snapshot: layout plus which tab was active. */
export interface SavedLayout {
  tabs: SavedTab[]
  activeIndex: number
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
  const workspaces = useWorkspacesStore()

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

  /**
   * Tabs belonging to the active workspace. Workspace-less tabs
   * (skipped onboarding, home-dir sessions) are always visible.
   * Hidden tabs keep their PTYs running — switching back restores
   * everything, scrollback included.
   */
  const visibleTabs = computed(() =>
    tabs.value.filter(t =>
      t.workspaceId === null || t.workspaceId === workspaces.activeId
    )
  )

  // switching workspace swaps the visible set; land on one of its tabs
  watch(() => workspaces.activeId, (workspaceId) => {
    if (activeTab.value && visibleTabs.value.some(t => t.id === activeTab.value!.id)) return
    const first = visibleTabs.value[0]
    if (first) {
      activeTabId.value = first.id
    } else if (workspaceId) {
      // empty workspace → start a session in it
      const workspace = workspaces.list.find(w => w.id === workspaceId)
      if (workspace) create(workspace.path, workspace.name)
      else activeTabId.value = null
    } else {
      activeTabId.value = null
    }
  })

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
  function create(cwd: string | null = null, title?: string, workspaceId?: string | null): TerminalTab {
    const session = newSession(cwd, title)
    const tab: TerminalTab = {
      id: crypto.randomUUID(),
      title: title ?? basename(cwd) ?? 'zsh',
      branch: null,
      root: { type: 'leaf', sessionId: session.id },
      // sessions belong to the workspace they were opened in
      workspaceId: workspaceId !== undefined ? workspaceId : (cwd ? workspaces.activeId : null)
    }
    tabs.value.push(tab)
    activeTabId.value = tab.id
    focusedByTab.value[tab.id] = session.id
    // Resolve the git branch for the sidebar (async, best-effort)
    if (cwd) resolveBranch(tab)
    return tab
  }

  /**
   * Splits a specific pane; the new pane opens in the origin's *live* cwd
   * (where its shell currently is), not the folder it was first spawned in.
   */
  async function splitPane(sessionId: string, direction: 'row' | 'column') {
    const tab = tabOf(sessionId)
    if (!tab) return
    await refreshCwd(sessionId)
    const origin = sessions.value[sessionId]
    const session = newSession(origin?.cwd ?? null, origin?.title)
    tab.root = insertAtLeaf(tab.root, sessionId, {
      type: 'leaf',
      sessionId: session.id
    }, direction)
    activeTabId.value = tab.id
    focusedByTab.value[tab.id] = session.id
  }

  /** Splits the focused pane of the active tab (⌘D / ⇧⌘D). */
  function split(direction: 'row' | 'column') {
    const focused = focusedSessionId.value
    if (focused) splitPane(focused, direction)
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

  /**
   * Re-resolves a tab's git branch from its focused pane's *live* cwd, so
   * the sidebar follows the user into sub-repos (e.g. a workspace root
   * `keloo-accounting` that's only a container shows the branch once you
   * `cd backend`). Falls back to the first pane when nothing is focused.
   */
  function resolveBranch(tab: TerminalTab) {
    const focusedId = focusedByTab.value[tab.id]
    const leaves = leavesOf(tab.root)
    const leafId = focusedId && leaves.some(l => l.sessionId === focusedId)
      ? focusedId
      : leaves[0]?.sessionId
    const cwd = leafId ? sessions.value[leafId]?.cwd ?? null : null
    if (!cwd) {
      tab.branch = null
      return
    }
    invoke<string | null>('git_branch', { path: cwd })
      .then((branch) => { tab.branch = branch })
      .catch(() => {})
  }

  /**
   * Pulls the live cwd of a session's shell from the OS (the user may have
   * `cd`'d since spawn). Updates the stored cwd and, when it's the pane the
   * tab's branch follows, re-resolves the branch.
   */
  async function refreshCwd(sessionId: string) {
    const session = sessions.value[sessionId]
    if (!session) return
    const cwd = await invoke<string | null>('pty_cwd', { id: sessionId }).catch(() => null)
    if (!cwd || cwd === session.cwd) return
    session.cwd = cwd
    const tab = tabOf(sessionId)
    if (!tab) return
    const leaves = leavesOf(tab.root)
    const follows = focusedByTab.value[tab.id] === sessionId
      || (!focusedByTab.value[tab.id] && leaves[0]?.sessionId === sessionId)
    if (follows) resolveBranch(tab)
  }

  /** Refreshes every live session's cwd — keeps branch + saved layout current. */
  async function refreshAllCwds() {
    await Promise.all(Object.keys(sessions.value).map(refreshCwd))
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
        activeTabId.value = visibleTabs.value[0]?.id ?? null
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
      activeTabId.value = visibleTabs.value[0]?.id ?? null
    }
  }

  function setActiveTab(id: string) {
    activeTabId.value = id
  }

  // --- session restore ------------------------------------------------------

  function serializeNode(node: PaneNode, focusedId: string | undefined): SavedNode {
    if (node.type === 'leaf') {
      const session = sessions.value[node.sessionId]
      return {
        type: 'leaf',
        cwd: session?.cwd ?? null,
        title: session?.title ?? 'zsh',
        focused: node.sessionId === focusedId
      }
    }
    return {
      type: 'split',
      direction: node.direction,
      sizes: node.sizes,
      children: [
        serializeNode(node.children[0], focusedId),
        serializeNode(node.children[1], focusedId)
      ]
    }
  }

  /** Layout snapshot (cwds + split tree + active tab + focus), safe to persist. */
  function serialize(): SavedLayout {
    return {
      tabs: tabs.value.map(tab => ({
        title: tab.title,
        root: serializeNode(tab.root, focusedByTab.value[tab.id]),
        workspaceId: tab.workspaceId
      })),
      activeIndex: tabs.value.findIndex(t => t.id === activeTabId.value)
    }
  }

  function restoreNode(node: SavedNode, onFocused: (sessionId: string) => void): PaneNode {
    if (node.type === 'leaf') {
      const session = newSession(node.cwd, node.title)
      if (node.focused) onFocused(session.id)
      return { type: 'leaf', sessionId: session.id }
    }
    return {
      type: 'split',
      id: crypto.randomUUID(),
      direction: node.direction,
      sizes: node.sizes,
      children: [
        restoreNode(node.children[0], onFocused),
        restoreNode(node.children[1], onFocused)
      ]
    }
  }

  /**
   * Rebuilds tabs from a snapshot — fresh PTYs spawn per pane on mount.
   * Accepts the current `SavedLayout` shape or a legacy bare `SavedTab[]`.
   */
  function restore(saved: SavedLayout | SavedTab[]) {
    const layout: SavedLayout = Array.isArray(saved)
      ? { tabs: saved, activeIndex: -1 }
      : saved
    // drop tabs whose workspace no longer exists
    const knownWorkspaces = new Set(workspaces.list.map(w => w.id))
    let activeId: string | null = null
    layout.tabs.forEach((savedTab, savedIndex) => {
      const workspaceId = savedTab.workspaceId ?? null
      if (workspaceId && !knownWorkspaces.has(workspaceId)) return
      let focusedLeaf: string | null = null
      const root = restoreNode(savedTab.root, (id) => { focusedLeaf = id })
      const tab: TerminalTab = {
        id: crypto.randomUUID(),
        title: savedTab.title,
        branch: null,
        root,
        workspaceId
      }
      tabs.value.push(tab)
      focusedByTab.value[tab.id] = focusedLeaf ?? leavesOf(root)[0]!.sessionId
      resolveBranch(tab)
      if (savedIndex === layout.activeIndex) activeId = tab.id
    })
    // land on the previously-active tab if it's visible, else the first
    const visible = visibleTabs.value
    activeTabId.value = (activeId && visible.some(t => t.id === activeId))
      ? activeId
      : visible[0]?.id ?? tabs.value[0]?.id ?? null
  }

  function focusPane(sessionId: string) {
    const tab = tabOf(sessionId)
    if (!tab) return
    activeTabId.value = tab.id
    focusedByTab.value[tab.id] = sessionId
    // branch + cockpit follow the focused pane's repo. Resolve from the
    // known cwd immediately, then pull the live cwd so switching to a pane
    // that `cd`'d elsewhere updates without waiting for the next poll.
    resolveBranch(tab)
    refreshCwd(sessionId)
  }

  return {
    sessions, tabs, visibleTabs, activeTabId, focusedByTab, draggingTabId, draggingSessionId,
    activeTab, focusedSessionId, paneCount,
    create, split, splitPane, moveTabIntoPane, movePaneIntoPane,
    closePane, kill, killTab, setActiveTab, focusPane,
    refreshCwd, refreshAllCwds,
    serialize, restore
  }
})
