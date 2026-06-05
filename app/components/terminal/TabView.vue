<script setup lang="ts">
import type { PaneSplit, TerminalTab } from '~/stores/terminals'
import { leavesOf } from '~/stores/terminals'

const props = defineProps<{ tab: TerminalTab }>()

const terminals = useTerminalsStore()
const el = ref<HTMLDivElement>()

interface Rect {
  x: number
  y: number
  w: number
  h: number
}

interface DividerHit {
  split: PaneSplit
  /** the region occupied by the whole split node, in container % */
  region: Rect
  /** boundary position in container % (x for row, y for column) */
  at: number
}

/**
 * Panes are rendered FLAT (absolutely positioned), never nested:
 * splitting only changes their computed rects, so existing xterm
 * instances are never remounted and scrollback survives.
 */
const layout = computed(() => {
  const leaves: Array<{ sessionId: string, rect: Rect }> = []
  const dividers: DividerHit[] = []

  function walk(node: TerminalTab['root'], rect: Rect) {
    if (node.type === 'leaf') {
      leaves.push({ sessionId: node.sessionId, rect })
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

  walk(props.tab.root, { x: 0, y: 0, w: 100, h: 100 })
  return { leaves, dividers }
})

const multiPane = computed(() => leavesOf(props.tab.root).length > 1)

function paneStyle(rect: Rect) {
  return {
    left: `${rect.x}%`,
    top: `${rect.y}%`,
    width: `${rect.w}%`,
    height: `${rect.h}%`
  }
}

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

const dragging = ref(false)

function startDrag(d: DividerHit, event: MouseEvent) {
  event.preventDefault()
  const container = el.value
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
</script>

<template>
  <div
    ref="el"
    class="relative h-full w-full bg-[#0d0d0d]"
    :class="dragging ? 'select-none' : ''"
  >
    <div
      v-for="pane in layout.leaves"
      :key="pane.sessionId"
      class="absolute p-px"
      :style="paneStyle(pane.rect)"
      @mousedown.capture="terminals.focusPane(pane.sessionId)"
    >
      <div
        class="h-full w-full overflow-hidden rounded-sm"
        :class="multiPane
          ? (terminals.focusedSessionId === pane.sessionId
            ? 'ring-1 ring-blue-500/50'
            : 'ring-1 ring-[#222222]')
          : ''"
      >
        <TerminalPane
          :session-id="pane.sessionId"
          :cwd="terminals.sessions[pane.sessionId]?.cwd ?? null"
          @exited="terminals.closePane(pane.sessionId)"
        />
      </div>
    </div>

    <div
      v-for="divider in layout.dividers"
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
  </div>
</template>
