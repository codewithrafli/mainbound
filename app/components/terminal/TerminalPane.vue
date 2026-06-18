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

interface SpeechRecognitionAlternativeLike {
  transcript: string
}

interface SpeechRecognitionResultLike {
  isFinal: boolean
  0?: SpeechRecognitionAlternativeLike
}

interface SpeechRecognitionResultListLike {
  length: number
  [index: number]: SpeechRecognitionResultLike
}

interface SpeechRecognitionEventLike {
  resultIndex: number
  results: SpeechRecognitionResultListLike
}

interface SpeechRecognitionLike {
  lang: string
  interimResults: boolean
  continuous: boolean
  maxAlternatives: number
  onstart: (() => void) | null
  onend: (() => void) | null
  onerror: ((event: { error?: string }) => void) | null
  onresult: ((event: SpeechRecognitionEventLike) => void) | null
  start: () => void
  stop: () => void
  abort: () => void
}

type SpeechRecognitionCtor = new () => SpeechRecognitionLike

const notifications = useNotificationsStore()
const toast = useToast()
const { settings } = useSettingsStore()
const { isLinux } = usePlatform()
const el = ref<HTMLDivElement>()
const dictating = ref(false)
const dictationPreview = ref('')

let term: Terminal | undefined
let fit: FitAddon | undefined
let search: SearchAddon | undefined
let recognition: SpeechRecognitionLike | undefined
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

  // Renderer: try WebGL → Canvas → DOM (in order of performance).
  // On Linux, SKIP WebGL entirely: the WebKitGTK WebGL path leaks GPU
  // memory catastrophically (WebKitWebProcess → tens of GB → OOM kill)
  // and mis-composites the layout. Canvas is stable there.
  let rendererLoaded = false
  if (!isLinux.value) {
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

  let spawnOk = false
  try {
    await invoke('pty_spawn', {
      id: props.sessionId,
      cwd: props.cwd,
      cols: term.cols,
      rows: term.rows,
      shell: settings.shell || null
    })
    spawnOk = true
  } catch (err) {
    term.writeln(`\x1b[31mfailed to spawn shell: ${err}\x1b[0m`)
    // Clean up listeners so they don't fire for a future session with same id
    unlisteners.forEach(off => off())
    unlisteners.length = 0
    return
  }
  if (!spawnOk) return

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

  // Image paste: terminal CLIs that accept image attachments (Codex,
  // Claude Code) detect an image file path in pasted input. When the
  // clipboard holds an image, save it to a temp file and paste the path
  // instead of letting xterm drop the binary data. (iTerm2/Ghostty/cmux
  // do the same.)
  el.value!.addEventListener('paste', onPaste, true)

  term.focus()
})

/** Intercept clipboard-image pastes; let normal text pastes through. */
async function onPaste(event: ClipboardEvent) {
  const items = event.clipboardData?.items
  if (!items) return

  const imageItem = Array.from(items).find(it => it.type.startsWith('image/'))
  if (!imageItem) return // plain text paste — xterm handles it

  // We're handling an image — stop xterm from processing the empty paste
  event.preventDefault()
  event.stopPropagation()

  const file = imageItem.getAsFile()
  if (!file) return

  try {
    const bytes = new Uint8Array(await file.arrayBuffer())
    const ext = imageItem.type.split('/')[1] || 'png'
    const path = await invoke<string>('save_clipboard_image', {
      data: Array.from(bytes),
      ext
    })
    // When the program has bracketed-paste mode on (Codex, Claude Code,
    // modern shells), wrap the path in paste markers so it's treated as a
    // paste — that's when image-path detection runs. Otherwise send the
    // raw path so we don't inject literal escape garbage.
    const bracketed = term?.modes.bracketedPasteMode
    const payload = bracketed ? `\x1b[200~${path}\x1b[201~` : path
    await invoke('pty_write', { id: props.sessionId, data: payload })
  } catch (err) {
    term?.writeln(`\r\n\x1b[31mimage paste failed: ${err}\x1b[0m`)
  }
}

function speechRecognitionCtor(): SpeechRecognitionCtor | null {
  const speechWindow = window as Window & {
    SpeechRecognition?: SpeechRecognitionCtor
    webkitSpeechRecognition?: SpeechRecognitionCtor
  }
  return speechWindow.SpeechRecognition ?? speechWindow.webkitSpeechRecognition ?? null
}

async function writePastedText(text: string) {
  if (!text) return
  const bracketed = term?.modes.bracketedPasteMode
  const payload = bracketed ? `\x1b[200~${text}\x1b[201~` : text
  await invoke('pty_write', { id: props.sessionId, data: payload })
}

function stopDictation() {
  recognition?.stop()
}

function toggleDictation() {
  if (dictating.value) {
    stopDictation()
    return
  }

  const Recognition = speechRecognitionCtor()
  if (!Recognition) {
    toast.add({
      title: 'Speech-to-text unavailable',
      description: 'This webview does not expose browser speech recognition.',
      icon: 'i-lucide-mic-off',
      color: 'warning'
    })
    return
  }

  recognition?.abort()
  dictationPreview.value = ''
  recognition = new Recognition()
  recognition.lang = settings.speechLanguage.trim() || navigator.language || 'en-US'
  recognition.interimResults = true
  recognition.continuous = false
  recognition.maxAlternatives = 1
  recognition.onstart = () => {
    dictating.value = true
  }
  recognition.onend = () => {
    dictating.value = false
    dictationPreview.value = ''
    recognition = undefined
  }
  recognition.onerror = (event) => {
    dictating.value = false
    dictationPreview.value = ''
    recognition = undefined
    toast.add({
      title: 'Dictation failed',
      description: event.error || 'Speech recognition stopped unexpectedly.',
      icon: 'i-lucide-mic-off',
      color: 'error'
    })
  }
  recognition.onresult = (event) => {
    let finalText = ''
    let interimText = ''
    for (let index = event.resultIndex; index < event.results.length; index += 1) {
      const result = event.results[index]
      const transcript = result?.[0]?.transcript ?? ''
      if (result?.isFinal) finalText += transcript
      else interimText += transcript
    }
    dictationPreview.value = interimText.trim()
    const text = finalText.trim()
    if (text) writePastedText(text).catch(() => {})
  }

  try {
    recognition.start()
  } catch (error) {
    recognition = undefined
    dictating.value = false
    toast.add({
      title: 'Dictation failed',
      description: String(error),
      icon: 'i-lucide-mic-off',
      color: 'error'
    })
  }
}

onBeforeUnmount(() => {
  recognition?.abort()
  el.value?.removeEventListener('paste', onPaste, true)
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

defineExpose({ focus, findNext, findPrevious, clearSearch, toggleDictation, dictating })
</script>

<template>
  <!-- padding on the wrapper so FitAddon measures the host correctly -->
  <div class="relative h-full w-full bg-[#0e0e10] px-3 py-2">
    <div
      ref="el"
      class="h-full w-full"
    />
    <div
      v-if="dictating"
      class="pointer-events-none absolute bottom-2 left-3 right-3 z-20 flex items-center gap-2 rounded-md border border-blue-500/40 bg-[#101828]/95 px-2.5 py-1.5 text-[11px] text-blue-100 shadow-lg"
    >
      <UIcon
        name="i-lucide-mic"
        class="size-3.5 text-blue-300"
      />
      <span class="shrink-0 font-medium">Listening</span>
      <span class="min-w-0 truncate font-mono text-blue-200/80">
        {{ dictationPreview || 'Speak now…' }}
      </span>
    </div>
  </div>
</template>
