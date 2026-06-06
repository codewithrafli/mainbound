<script setup lang="ts">
import type { PaneNode, PaneSplit } from '~/stores/terminals'
import { leavesOf } from '~/stores/terminals'

const terminals = useTerminalsStore()
const workspaces = useWorkspacesStore()

// Restore workspaces, then open a first session in the active one
onMounted(async () => {
  await workspaces.init()
  if (!terminals.tabs.length) {
    terminals.create(workspaces.active?.path ?? null, workspaces.active?.name)
  }
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
  <div class="flex h-full min-h-0">
    <TerminalSessionSidebar />

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
        <div
          v-for="pane in layout.panes"
          :key="pane.sessionId"
          class="absolute p-px group/pane"
          :style="paneStyle(pane)"
          @mousedown.capture="terminals.focusPane(pane.sessionId)"
        >
          <div
            class="h-full w-full overflow-hidden rounded-sm"
            :class="paneRing(pane)"
          >
            <TerminalPane
              :session-id="pane.sessionId"
              :cwd="terminals.sessions[pane.sessionId]?.cwd ?? null"
              @exited="terminals.closePane(pane.sessionId)"
            />
          </div>

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

          <!-- close pane (hover) -->
          <button
            class="absolute top-1 right-1.5 z-10 flex items-center justify-center size-4 rounded
                 bg-[#1a1a1a] text-dimmed hover:text-red-400 hover:bg-[#222222]
                 opacity-0 group-hover/pane:opacity-100 transition-opacity"
            title="Close pane (⌘W)"
            @click.stop="terminals.kill(pane.sessionId)"
          >
            <UIcon
              name="i-lucide-x"
              class="size-3"
            />
          </button>
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
          v-if="!terminals.tabs.length"
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
