<script setup lang="ts">
import { Terminal } from '@xterm/xterm'
import { FitAddon } from '@xterm/addon-fit'
import { WebglAddon } from '@xterm/addon-webgl'
import { Unicode11Addon } from '@xterm/addon-unicode11'
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
const el = ref<HTMLDivElement>()

let term: Terminal | undefined
let fit: FitAddon | undefined
let resizeObserver: ResizeObserver | undefined
const unlisteners: UnlistenFn[] = []

onMounted(async () => {
  term = new Terminal({
    // required by the unicode11 addon
    allowProposedApi: true,
    cursorBlink: true,
    scrollback: 10_000,
    fontSize: 12.5,
    fontFamily: 'ui-monospace, "SF Mono", Menlo, monospace',
    lineHeight: 1.35,
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
  // match zsh's wcwidth: emoji & friends count as 2 cells, otherwise
  // p10k-style prompts position the cursor past the rendered text
  term.loadAddon(new Unicode11Addon())
  term.unicode.activeVersion = '11'
  term.open(el.value!)
  try {
    term.loadAddon(new WebglAddon())
  } catch {
    // WebGL unavailable — xterm falls back to the DOM renderer
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
      rows: term.rows
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

defineExpose({ focus })
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
