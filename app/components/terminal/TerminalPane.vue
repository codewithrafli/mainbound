<script setup lang="ts">
import { Terminal } from '@xterm/xterm'
import { FitAddon } from '@xterm/addon-fit'
import { WebglAddon } from '@xterm/addon-webgl'
import { CanvasAddon } from '@xterm/addon-canvas'
import { Unicode11Addon } from '@xterm/addon-unicode11'
import { SearchAddon } from '@xterm/addon-search'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'

const props = defineProps<{
  sessionId: string
  cwd: string | null
}>()

const emit = defineEmits<{
  exited: [code: number]
}>()

const notifications = useNotificationsStore()
const { settings } = useSettingsStore()
const el = ref<HTMLDivElement>()

let term: Terminal | undefined
let fit: FitAddon | undefined
let search: SearchAddon | undefined
let resizeObserver: ResizeObserver | undefined
const unlisteners: UnlistenFn[] = []

// live font-size from settings
watch(() => settings.fontSize, (size) => {
  if (!term) return
  term.options.fontSize = size
  fit?.fit()
})

onMounted(async () => {
  // Cross-platform monospace stack: Consolas ships with Windows,
  // SF Mono/Menlo on macOS, Noto on Linux
  const fontFamily = 'ui-monospace, "Cascadia Code", Consolas, "SF Mono", Menlo, "DejaVu Sans Mono", monospace'

  term = new Terminal({
    allowProposedApi: true,
    cursorBlink: true,
    scrollback: 10_000,
    fontSize: settings.fontSize,
    fontFamily,
    lineHeight: 1.35,
    // Disable GPU compositing on Windows to avoid WebView2 flickering
    fastScrollModifier: 'alt',
    theme: {
      background: '#0e0e10',
      foreground: '#cccccc',
      cursor: '#cccccc',
      selectionBackground: '#264f78',
      black: '#1a1a1a',
      brightBlack: '#555555'
    }
  })
  fit = new FitAddon()
  term.loadAddon(fit)
  term.loadAddon(new Unicode11Addon())
  term.unicode.activeVersion = '11'
  search = new SearchAddon()
  term.loadAddon(search)
  term.open(el.value!)

  // Renderer: try WebGL → Canvas → DOM (in order of performance)
  let rendererLoaded = false
  try {
    const webgl = new WebglAddon()
    // WebGL can throw or silently fail on some Windows GPU drivers
    webgl.onContextLoss(() => {
      webgl.dispose()
    })
    term.loadAddon(webgl)
    rendererLoaded = true
  } catch {
    // WebGL failed — try Canvas
  }
  if (!rendererLoaded) {
    try {
      term.loadAddon(new CanvasAddon())
    } catch {
      // Fall back to DOM renderer (always works, slowest)
    }
  }

  fit.fit()

  // notification signals: bell + explicit OSC notifications
  term.onBell(() => notifications.onBell(props.sessionId))
  term.parser.registerOscHandler(9, (data) => {
    notifications.onOscNotify(props.sessionId, '', data)
    return true
  })
  term.parser.registerOscHandler(777, (data) => {
    // urxvt format: notify;title;body
    const [kind, title, ...rest] = data.split(';')
    if (kind === 'notify') notifications.onOscNotify(props.sessionId, title ?? '', rest.join(';'))
    return true
  })

  // Listeners BEFORE spawn so the first prompt bytes aren't lost
  unlisteners.push(
    await listen<string>(`pty://data/${props.sessionId}`, (e) => {
      term?.write(e.payload)
      notifications.onOutput(props.sessionId)
    }),
    await listen<{ id: string, code: number }>(`pty://exit/${props.sessionId}`, (e) => {
      emit('exited', e.payload.code)
    })
  )

  try {
    await invoke('pty_spawn', {
      id: props.sessionId,
      cwd: props.cwd,
      cols: term.cols,
      rows: term.rows,
      shell: settings.shell || null
    })
  } catch (err) {
    term.writeln(`\x1b[31mfailed to spawn shell: ${err}\x1b[0m`)
    return
  }

  term.onData((data) => {
    invoke('pty_write', { id: props.sessionId, data })
  })
  term.onResize(({ cols, rows }) => {
    invoke('pty_resize', { id: props.sessionId, cols, rows })
  })

  // Layout may settle a frame after mount (fonts, frame chrome) and
  // term.onResize only fires when dims CHANGE — force one authoritative
  // sync so the PTY always matches the rendered geometry.
  requestAnimationFrame(() => {
    if (!term || !fit) return
    fit.fit()
    invoke('pty_resize', { id: props.sessionId, cols: term.cols, rows: term.rows })
  })

  // Refit on container resize — also fires when the pane becomes
  // visible again after a v-show toggle
  resizeObserver = new ResizeObserver(() => {
    if (el.value && el.value.offsetWidth > 0) fit?.fit()
  })
  resizeObserver.observe(el.value!)

  term.focus()
})

onBeforeUnmount(() => {
  resizeObserver?.disconnect()
  unlisteners.forEach(off => off())
  notifications.forget(props.sessionId)
  term?.dispose()
})

function focus() {
  term?.focus()
}

const SEARCH_DECOR = {
  matchBackground: '#3b3b7a',
  matchBorder: '#3b82f6',
  matchOverviewRuler: '#3b82f6',
  activeMatchBackground: '#b45309',
  activeMatchBorder: '#f59e0b',
  activeMatchColorOverviewRuler: '#f59e0b'
}

function findNext(query: string, incremental = false) {
  search?.findNext(query, { incremental, decorations: SEARCH_DECOR })
}

function findPrevious(query: string) {
  search?.findPrevious(query, { decorations: SEARCH_DECOR })
}

function clearSearch() {
  search?.clearDecorations()
}

defineExpose({ focus, findNext, findPrevious, clearSearch })
</script>

<template>
  <!-- padding on the wrapper so FitAddon measures the host correctly -->
  <div class="h-full w-full bg-[#0e0e10] px-3 py-2">
    <div
      ref="el"
      class="h-full w-full"
    />
  </div>
</template>
