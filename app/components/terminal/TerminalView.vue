<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import type { PaneNode, PaneSplit, SavedLayout, SavedTab } from '~/stores/terminals'
import { leavesOf } from '~/stores/terminals'

const terminals = useTerminalsStore()
const workspaces = useWorkspacesStore()
const ui = useUiStore()

// Restore workspaces, then previous session layout (or a fresh session)
onMounted(async () => {
  await workspaces.init()
  if (terminals.tabs.length) return // HMR remount — already live
  const saved = await invoke<SavedLayout | SavedTab[] | null>('sessions_load').catch(() => null)
  const savedTabs = Array.isArray(saved) ? saved : saved?.tabs
  if (saved && savedTabs?.length) {
    terminals.restore(saved)
  } else if (workspaces.list.length) {
    terminals.create(workspaces.active?.path ?? null, workspaces.active?.name)
  }
  // no workspaces yet → the onboarding overlay takes over
})

// --- ⌘F search (acts on the focused pane) ---------------------------------

interface PaneHandle {
  findNext: (q: string, incremental?: boolean) => void
  findPrevious: (q: string) => void
  clearSearch: () => void
  focus: () => void
  toggleDictation: () => void
}

const paneRefs = new Map<string, PaneHandle>()
const paneDictation = reactive<Record<string, { dictating: boolean, transcribing: boolean }>>({})
const searchInput = ref()
const searchQuery = ref('')

function setPaneRef(sessionId: string, handle: unknown) {
  if (handle) paneRefs.set(sessionId, handle as PaneHandle)
  else {
    paneRefs.delete(sessionId)
    paneDictation[sessionId] = { dictating: false, transcribing: false }
  }
}

function focusedPane(): PaneHandle | undefined {
  const id = terminals.focusedSessionId
  return id ? paneRefs.get(id) : undefined
}

watch(() => ui.terminalSearchOpen, async (open) => {
  if (open) {
    await nextTick()
    searchInput.value?.inputRef?.focus()
  } else {
    focusedPane()?.clearSearch()
    searchQuery.value = ''
    focusedPane()?.focus()
  }
})

// Clear search when switching tabs so it doesn't bleed into another session
watch(() => terminals.activeTabId, () => {
  if (ui.terminalSearchOpen) {
    focusedPane()?.clearSearch()
    searchQuery.value = ''
  }
})

watch(searchQuery, (q) => {
  if (q) focusedPane()?.findNext(q, true)
  else focusedPane()?.clearSearch()
})

function searchStep(backwards = false) {
  const q = searchQuery.value
  if (!q) return
  if (backwards) focusedPane()?.findPrevious(q)
  else focusedPane()?.findNext(q)
}

// right-click menu: split/close belong to the pane under the cursor
function paneMenuItems(sessionId: string) {
  return [[
    {
      label: 'Split Right',
      icon: 'i-lucide-columns-2',
      kbds: ['meta', 'D'],
      onSelect: () => terminals.splitPane(sessionId, 'row')
    },
    {
      label: 'Split Down',
      icon: 'i-lucide-rows-2',
      kbds: ['shift', 'meta', 'D'],
      onSelect: () => terminals.splitPane(sessionId, 'column')
    }
  ], [
    {
      label: 'Close Pane',
      icon: 'i-lucide-x',
      kbds: ['meta', 'W'],
      color: 'error' as const,
      onSelect: () => terminals.kill(sessionId)
    }
  ]]
}

// Persist the layout. Debounced for frequent changes, but always flushed
// immediately when the window closes / the app quits — otherwise a pending
// debounce is lost on shutdown and the layout "resets" on next launch.
let saveTimer: ReturnType<typeof setTimeout> | undefined

function flushSave() {
  clearTimeout(saveTimer)
  saveTimer = undefined
  // fire-and-forget; the Rust side writes to disk synchronously
  invoke('sessions_save', { data: terminals.serialize() }).catch(() => {})
}

watch(() => terminals.tabs, () => {
  clearTimeout(saveTimer)
  saveTimer = setTimeout(flushSave, 1_000)
}, { deep: true })

// Flush on quit/close. Tauri fires onCloseRequested on Cmd+Q / window close
// and on OS shutdown; the webview also fires pagehide. Cover both.
let unlistenClose: (() => void) | undefined

// Poll the live cwd of every shell so the git-branch in the sidebar follows
// `cd`, and so the persisted layout reflects where each pane actually is.
let cwdTimer: ReturnType<typeof setInterval> | undefined

onMounted(async () => {
  // save when the webview is being torn down (covers reload + some quits)
  window.addEventListener('pagehide', flushSave)
  window.addEventListener('beforeunload', flushSave)

  cwdTimer = setInterval(() => {
    terminals.refreshAllCwds()
  }, 2_000)

  const { getCurrentWindow } = await import('@tauri-apps/api/window')
  const win = getCurrentWindow()
  unlistenClose = await win.onCloseRequested(async (event) => {
    // hold the close so the synchronous disk write completes, then destroy
    event.preventDefault()
    try {
      await terminals.refreshAllCwds()
      flushSave()
      const { saveWindowState, StateFlags } = await import('@tauri-apps/plugin-window-state')
      await saveWindowState(StateFlags.ALL)
    } catch { /* never block the close on a save error */ }
    await win.destroy()
  })
})

onBeforeUnmount(() => {
  window.removeEventListener('pagehide', flushSave)
  window.removeEventListener('beforeunload', flushSave)
  clearInterval(cwdTimer)
  unlistenClose?.()
})

const area = ref<HTMLDivElement>()

interface Rect {
  x: number
  y: number
  w: number
  h: number
}

interface PaneBox {
  sessionId: string
  tabId: string
  rect: Rect
}

interface DividerHit {
  split: PaneSplit
  region: Rect
  at: number
}

/**
 * Panes of ALL tabs are rendered in ONE flat keyed list (absolutely
 * positioned, inactive tabs hidden via display:none): splitting or
 * moving a pane between tabs only changes its rect/visibility, so the
 * underlying xterm is never remounted and PTY + scrollback survive.
 */
const layout = computed(() => {
  const panes: PaneBox[] = []
  const dividersByTab: Record<string, DividerHit[]> = {}

  for (const tab of terminals.tabs) {
    const dividers: DividerHit[] = []
    const walk = (node: PaneNode, rect: Rect) => {
      if (node.type === 'leaf') {
        panes.push({ sessionId: node.sessionId, tabId: tab.id, rect })
        return
      }
      const ratio = node.sizes[0] / 100
      if (node.direction === 'row') {
        const w0 = rect.w * ratio
        walk(node.children[0], { ...rect, w: w0 })
        dividers.push({ split: node, region: rect, at: rect.x + w0 })
        walk(node.children[1], { ...rect, x: rect.x + w0, w: rect.w - w0 })
      } else {
        const h0 = rect.h * ratio
        walk(node.children[0], { ...rect, h: h0 })
        dividers.push({ split: node, region: rect, at: rect.y + h0 })
        walk(node.children[1], { ...rect, y: rect.y + h0, h: rect.h - h0 })
      }
    }
    walk(tab.root, { x: 0, y: 0, w: 100, h: 100 })
    dividersByTab[tab.id] = dividers
  }

  return { panes, dividersByTab }
})

const activeDividers = computed(() =>
  terminals.activeTabId ? layout.value.dividersByTab[terminals.activeTabId] ?? [] : []
)

const activeMultiPane = computed(() => {
  const tab = terminals.activeTab
  return tab ? leavesOf(tab.root).length > 1 : false
})

function paneStyle(pane: PaneBox) {
  return {
    left: `${pane.rect.x}%`,
    top: `${pane.rect.y}%`,
    width: `${pane.rect.w}%`,
    height: `${pane.rect.h}%`,
    display: pane.tabId === terminals.activeTabId ? undefined : 'none'
  }
}

function paneRing(pane: PaneBox) {
  if (!activeMultiPane.value || pane.tabId !== terminals.activeTabId) return ''
  return terminals.focusedSessionId === pane.sessionId
    ? 'ring-1 ring-blue-500/50'
    : 'ring-1 ring-[#222222]'
}

function paneDictationState(sessionId: string) {
  return paneDictation[sessionId] ?? { dictating: false, transcribing: false }
}

function setPaneDictationState(sessionId: string, state: { dictating: boolean, transcribing: boolean }) {
  paneDictation[sessionId] = state
}

function togglePaneDictation(sessionId: string) {
  paneRefs.get(sessionId)?.toggleDictation()
}

// --- divider drag-resize -------------------------------------------------

const dragging = ref(false)

function dividerStyle(d: DividerHit) {
  if (d.split.direction === 'row') {
    return {
      left: `calc(${d.at}% - 3px)`,
      top: `${d.region.y}%`,
      width: '6px',
      height: `${d.region.h}%`
    }
  }
  return {
    left: `${d.region.x}%`,
    top: `calc(${d.at}% - 3px)`,
    width: `${d.region.w}%`,
    height: '6px'
  }
}

function startDrag(d: DividerHit, event: MouseEvent) {
  event.preventDefault()
  const container = area.value
  if (!container) return
  dragging.value = true
  const box = container.getBoundingClientRect()

  function onMove(ev: MouseEvent) {
    const cursorPct = d.split.direction === 'row'
      ? ((ev.clientX - box.left) / box.width) * 100
      : ((ev.clientY - box.top) / box.height) * 100
    const within = d.split.direction === 'row'
      ? ((cursorPct - d.region.x) / d.region.w) * 100
      : ((cursorPct - d.region.y) / d.region.h) * 100
    const first = Math.min(85, Math.max(15, within))
    d.split.sizes = [first, 100 - first]
  }

  function onUp() {
    dragging.value = false
    document.removeEventListener('mousemove', onMove)
    document.removeEventListener('mouseup', onUp)
  }

  document.addEventListener('mousemove', onMove)
  document.addEventListener('mouseup', onUp)
}

// --- drag & drop docking (sidebar tab -> pane edge) ----------------------

type DropZone = 'left' | 'right' | 'top' | 'bottom'

const dropTarget = ref<{ sessionId: string, zone: DropZone, rect: Rect } | null>(null)

function onDragOver(event: DragEvent) {
  const dragTab = terminals.draggingTabId
  const dragPane = terminals.draggingSessionId
  const container = area.value
  if ((!dragTab && !dragPane) || !container) return
  event.preventDefault()
  if (event.dataTransfer) event.dataTransfer.dropEffect = 'move'

  const box = container.getBoundingClientRect()
  const xPct = ((event.clientX - box.left) / box.width) * 100
  const yPct = ((event.clientY - box.top) / box.height) * 100

  const pane = layout.value.panes.find(p =>
    p.tabId === terminals.activeTabId
    && xPct >= p.rect.x && xPct <= p.rect.x + p.rect.w
    && yPct >= p.rect.y && yPct <= p.rect.y + p.rect.h
  )
  // a tab can't dock into itself; a pane can't dock onto itself
  if (!pane || (dragTab && pane.tabId === dragTab) || (dragPane && pane.sessionId === dragPane)) {
    dropTarget.value = null
    return
  }

  const rx = (xPct - pane.rect.x) / pane.rect.w
  const ry = (yPct - pane.rect.y) / pane.rect.h
  const distances: Array<[DropZone, number]> = [
    ['left', rx],
    ['right', 1 - rx],
    ['top', ry],
    ['bottom', 1 - ry]
  ]
  distances.sort((a, b) => a[1] - b[1])
  dropTarget.value = { sessionId: pane.sessionId, zone: distances[0]![0], rect: pane.rect }
}

function onDragLeave(event: DragEvent) {
  const related = event.relatedTarget as Node | null
  if (!related || !area.value?.contains(related)) {
    dropTarget.value = null
  }
}

function onDrop(event: DragEvent) {
  event.preventDefault()
  const target = dropTarget.value
  if (target && terminals.draggingSessionId) {
    terminals.movePaneIntoPane(terminals.draggingSessionId, target.sessionId, target.zone)
  } else if (target && terminals.draggingTabId) {
    terminals.moveTabIntoPane(terminals.draggingTabId, target.sessionId, target.zone)
  }
  dropTarget.value = null
  terminals.draggingTabId = null
  terminals.draggingSessionId = null
}

function onPaneDragStart(sessionId: string, event: DragEvent) {
  terminals.draggingSessionId = sessionId
  if (event.dataTransfer) {
    // WebKit requires data for the drag to start
    event.dataTransfer.setData('text/plain', sessionId)
    event.dataTransfer.effectAllowed = 'move'
  }
}

const dropOverlayStyle = computed(() => {
  const target = dropTarget.value
  if (!target) return {}
  const { rect, zone } = target
  const half: Rect = zone === 'left'
    ? { ...rect, w: rect.w / 2 }
    : zone === 'right'
      ? { ...rect, x: rect.x + rect.w / 2, w: rect.w / 2 }
      : zone === 'top'
        ? { ...rect, h: rect.h / 2 }
        : { ...rect, y: rect.y + rect.h / 2, h: rect.h / 2 }
  return {
    left: `${half.x}%`,
    top: `${half.y}%`,
    width: `${half.w}%`,
    height: `${half.h}%`
  }
})
</script>

<template>
  <div
    class="flex h-full min-h-0 min-w-0"
    style="min-width: 0;"
  >
    <TerminalSessionSidebar v-show="ui.leftSidebarOpen" />

    <div class="flex flex-col flex-1 min-w-0">
      <!-- PR Cockpit: repo state for the focused session, always visible -->
      <TerminalCockpitBar />

      <div
        ref="area"
        class="relative flex-1 min-h-0 bg-[#0e0e10]"
        :class="dragging ? 'select-none' : ''"
        @dragover="onDragOver"
        @dragleave="onDragLeave"
        @drop="onDrop"
      >
        <!-- ⌘F search bar -->
        <div
          v-if="ui.terminalSearchOpen"
          class="absolute top-2 right-3 z-30 flex items-center gap-1 rounded-lg border border-(--ui-border-accented) bg-elevated shadow-lg p-1"
        >
          <UInput
            ref="searchInput"
            v-model="searchQuery"
            placeholder="Find…"
            size="xs"
            class="w-44"
            :ui="{ base: 'font-mono' }"
            @keydown.enter.exact="searchStep()"
            @keydown.shift.enter="searchStep(true)"
            @keydown.esc="ui.terminalSearchOpen = false"
          />
          <UButton
            icon="i-lucide-chevron-up"
            color="neutral"
            variant="ghost"
            size="xs"
            aria-label="Previous match"
            @click="searchStep(true)"
          />
          <UButton
            icon="i-lucide-chevron-down"
            color="neutral"
            variant="ghost"
            size="xs"
            aria-label="Next match"
            @click="searchStep()"
          />
          <UButton
            icon="i-lucide-x"
            color="neutral"
            variant="ghost"
            size="xs"
            aria-label="Close search"
            @click="ui.terminalSearchOpen = false"
          />
        </div>
        <div
          v-for="pane in layout.panes"
          :key="pane.sessionId"
          class="absolute p-px group/pane"
          :style="paneStyle(pane)"
          @mousedown.capture="terminals.focusPane(pane.sessionId)"
        >
          <UContextMenu :items="paneMenuItems(pane.sessionId)">
            <div
              class="h-full w-full overflow-hidden rounded-sm"
              :class="paneRing(pane)"
            >
              <TerminalPane
                :ref="(handle) => setPaneRef(pane.sessionId, handle)"
                :session-id="pane.sessionId"
                :cwd="terminals.sessions[pane.sessionId]?.cwd ?? null"
                @exited="terminals.closePane(pane.sessionId)"
                @dictation-state="setPaneDictationState(pane.sessionId, $event)"
              />
            </div>
          </UContextMenu>

          <!-- grab handle: drag a pane to re-dock it on another pane's edge -->
          <div
            draggable="true"
            class="absolute top-0.5 left-1/2 -translate-x-1/2 z-10 flex items-center justify-center
                 w-10 h-4 rounded-b-md cursor-grab active:cursor-grabbing
                 bg-[#1a1a1a] text-dimmed opacity-0 group-hover/pane:opacity-100 transition-opacity"
            :class="terminals.draggingSessionId === pane.sessionId ? 'opacity-100 text-blue-400' : ''"
            title="Drag to move pane"
            @dragstart="onPaneDragStart(pane.sessionId, $event)"
            @dragend="terminals.draggingSessionId = null"
          >
            <UIcon
              name="i-lucide-grip-horizontal"
              class="size-3"
            />
          </div>

          <!-- pane actions (hover): split + close — splits belong to THIS pane -->
          <div
            class="absolute top-1 right-1.5 z-10 flex items-center gap-0.5 transition-opacity"
            :class="paneDictationState(pane.sessionId).dictating || paneDictationState(pane.sessionId).transcribing
              ? 'opacity-100'
              : 'opacity-0 group-hover/pane:opacity-100'"
          >
            <button
              class="flex items-center justify-center size-4 rounded bg-[#1a1a1a] text-dimmed hover:text-blue-300 hover:bg-[#222222]"
              :class="paneDictationState(pane.sessionId).dictating || paneDictationState(pane.sessionId).transcribing
                ? 'text-blue-300 bg-[#222222]'
                : ''"
              title="Dictate"
              @click.stop="togglePaneDictation(pane.sessionId)"
            >
              <UIcon
                :name="paneDictationState(pane.sessionId).transcribing
                  ? 'i-lucide-loader-circle'
                  : (paneDictationState(pane.sessionId).dictating ? 'i-lucide-square' : 'i-lucide-mic')"
                class="size-3"
                :class="paneDictationState(pane.sessionId).transcribing ? 'animate-spin' : ''"
              />
            </button>
            <button
              class="flex items-center justify-center size-4 rounded bg-[#1a1a1a] text-dimmed hover:text-toned hover:bg-[#222222]"
              title="Split right (⌘D)"
              @click.stop="terminals.splitPane(pane.sessionId, 'row')"
            >
              <UIcon
                name="i-lucide-columns-2"
                class="size-3"
              />
            </button>
            <button
              class="flex items-center justify-center size-4 rounded bg-[#1a1a1a] text-dimmed hover:text-toned hover:bg-[#222222]"
              title="Split down (⇧⌘D)"
              @click.stop="terminals.splitPane(pane.sessionId, 'column')"
            >
              <UIcon
                name="i-lucide-rows-2"
                class="size-3"
              />
            </button>
            <button
              class="flex items-center justify-center size-4 rounded bg-[#1a1a1a] text-dimmed hover:text-red-400 hover:bg-[#222222]"
              title="Close pane (⌘W)"
              @click.stop="terminals.kill(pane.sessionId)"
            >
              <UIcon
                name="i-lucide-x"
                class="size-3"
              />
            </button>
          </div>
        </div>

        <div
          v-for="divider in activeDividers"
          :key="divider.split.id"
          class="absolute z-10 group"
          :class="divider.split.direction === 'row' ? 'cursor-col-resize' : 'cursor-row-resize'"
          :style="dividerStyle(divider)"
          @mousedown="startDrag(divider, $event)"
        >
          <div
            class="bg-transparent group-hover:bg-blue-500/40 transition-colors"
            :class="divider.split.direction === 'row'
              ? 'mx-auto h-full w-px'
              : 'my-auto w-full h-px mt-[2.5px]'"
          />
        </div>

        <!-- drop-zone highlight while dragging a session from the sidebar -->
        <div
          v-if="dropTarget"
          class="absolute z-20 pointer-events-none rounded-md border-2 border-blue-500 bg-blue-500/15 transition-all duration-75"
          :style="dropOverlayStyle"
        />

        <div
          v-if="!terminals.visibleTabs.length"
          class="absolute inset-0 flex items-center justify-center"
        >
          <div class="panel-card flex flex-col items-center gap-3 px-12 py-12 rounded-2xl">
            <div class="flex items-center justify-center size-10 rounded-xl bg-elevated border border-default">
              <UIcon
                name="i-lucide-terminal"
                class="size-5 text-dimmed"
              />
            </div>
            <p class="text-sm text-muted">
              No open sessions
            </p>
            <UButton
              label="New Session"
              icon="i-lucide-plus"
              color="neutral"
              variant="solid"
              size="sm"
              @click="terminals.create(workspaces.active?.path ?? null, workspaces.active?.name)"
            />
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
