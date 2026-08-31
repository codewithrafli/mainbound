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
  dictationState: [state: { dictating: boolean, transcribing: boolean }]
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
const SILENCE_COMMIT_MS = 2_800
const GROQ_VOICE_THRESHOLD = 0.045
const GROQ_MIN_AUTO_SPEECH_MS = 450
const GROQ_SPECTRUM_BARS = 18
const DICTATION_DOCK_STORAGE_KEY = 'mainbound.dictationDockPosition'
const DICTATION_DOCK_MARGIN = 14
const TERMINAL_THEME = {
  background: '#0e0e10',
  foreground: '#cccccc',
  selectionBackground: '#264f78',
  black: '#1a1a1a',
  brightBlack: '#555555'
}

const notifications = useNotificationsStore()
const toast = useToast()
const { settings } = useSettingsStore()
const { isLinux } = usePlatform()
const host = ref<HTMLDivElement>()
const el = ref<HTMLDivElement>()
const dictating = ref(false)
const dictationPreview = ref('')
const transcribing = ref(false)
const groqSpectrum = ref(Array.from({ length: GROQ_SPECTRUM_BARS }, () => 0.18))
const dictationDock = reactive({
  ready: false,
  dragging: false,
  moved: false,
  x: 0,
  y: 0,
  startX: 0,
  startY: 0
})

let term: Terminal | undefined
let fit: FitAddon | undefined
let search: SearchAddon | undefined
let recognition: SpeechRecognitionLike | undefined
let silenceTimer: ReturnType<typeof setTimeout> | undefined
let pendingTranscript = ''
let transcriptBuffer = ''
let committedUtterance = false
let stoppingDictation = false
let mediaRecorder: MediaRecorder | undefined
let mediaStream: MediaStream | undefined
let audioContext: AudioContext | undefined
let analyser: AnalyserNode | undefined
let frequencyData: Uint8Array<ArrayBuffer> | undefined
let silenceFrame = 0
let speechStartedAt: number | null = null
let lastSpeechAt: number | null = null
let manualGroqStop = false
let recordedChunks: Blob[] = []
let resizeObserver: ResizeObserver | undefined
const unlisteners: UnlistenFn[] = []

const dictationDockStyle = computed(() => ({
  left: `${dictationDock.x}px`,
  top: `${dictationDock.y}px`,
  opacity: dictationDock.ready ? '1' : '0',
  transform: 'translate(-50%, -50%)'
}))

const dictationDockButtonClass = computed(() => [
  dictating.value
    ? (settings.speechProvider === 'groq'
        ? 'w-60 border-blue-400/60 bg-[#303033]/95 px-3.5 text-blue-100'
        : 'min-w-80 max-w-[min(34rem,calc(100vw-4rem))] border-blue-400/60 bg-[#303033]/95 px-3.5 text-blue-100')
    : 'w-28 border-white/10 bg-[#303033]/90 text-dimmed opacity-90 hover:text-toned hover:bg-[#38383b]/95',
  dictationDock.dragging ? 'cursor-grabbing' : 'cursor-grab'
])

// live font-size from settings
watch(() => settings.fontSize, (size) => {
  if (!term) return
  term.options.fontSize = size
  fit?.fit()
})

watch([dictating, () => settings.speechProvider], () => {
  nextTick(clampDictationDock)
})

watch([dictating, transcribing], () => {
  applyTerminalCursorState()
  emit('dictationState', {
    dictating: dictating.value,
    transcribing: transcribing.value
  })
})

function applyTerminalCursorState() {
  if (!term) return
  term.options.theme = {
    ...TERMINAL_THEME,
    cursor: transcribing.value ? '#f59e0b' : (dictating.value ? '#60a5fa' : '#cccccc'),
    cursorAccent: '#0e0e10'
  }
}

onMounted(async () => {
  // Cross-platform monospace stack: Consolas ships with Windows,
  // SF Mono/Menlo on macOS, Noto on Linux
  const fontFamily = 'ui-monospace, "Cascadia Code", Consolas, "SF Mono", Menlo, "DejaVu Sans Mono", monospace'

  const terminalOptions = {
    allowProposedApi: true,
    cursorBlink: true,
    scrollback: 10_000,
    fontSize: settings.fontSize,
    fontFamily,
    lineHeight: 1.35,
    // Disable GPU compositing on Windows to avoid WebView2 flickering
    fastScrollModifier: 'alt',
    theme: {
      ...TERMINAL_THEME,
      cursor: '#cccccc',
      cursorAccent: '#0e0e10'
    }
  } as ConstructorParameters<typeof Terminal>[0] & { fastScrollModifier: 'alt' }
  term = new Terminal(terminalOptions)
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
    clampDictationDock()
  })
  resizeObserver.observe(el.value!)
  requestAnimationFrame(initializeDictationDock)

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

function defaultDockPosition() {
  const rect = host.value?.getBoundingClientRect()
  return {
    x: rect ? rect.width / 2 : 0,
    y: rect ? Math.max(DICTATION_DOCK_MARGIN, rect.height - 56) : 0
  }
}

function clampDockPosition(x: number, y: number) {
  const rect = host.value?.getBoundingClientRect()
  if (!rect) return { x, y }
  const halfWidth = dictating.value && settings.speechProvider !== 'groq' ? 170 : 130
  const halfHeight = 30
  return {
    x: Math.min(Math.max(x, DICTATION_DOCK_MARGIN + halfWidth), Math.max(DICTATION_DOCK_MARGIN + halfWidth, rect.width - DICTATION_DOCK_MARGIN - halfWidth)),
    y: Math.min(Math.max(y, DICTATION_DOCK_MARGIN + halfHeight), Math.max(DICTATION_DOCK_MARGIN + halfHeight, rect.height - DICTATION_DOCK_MARGIN - halfHeight))
  }
}

function saveDictationDock() {
  if (!import.meta.client) return
  localStorage.setItem(DICTATION_DOCK_STORAGE_KEY, JSON.stringify({
    x: dictationDock.x,
    y: dictationDock.y
  }))
}

function initializeDictationDock() {
  const fallback = defaultDockPosition()
  let position = fallback
  if (import.meta.client) {
    try {
      const saved = JSON.parse(localStorage.getItem(DICTATION_DOCK_STORAGE_KEY) || 'null') as { x?: number, y?: number } | null
      if (typeof saved?.x === 'number' && typeof saved.y === 'number') {
        position = { x: saved.x, y: saved.y }
      }
    } catch {
      position = fallback
    }
  }
  const clamped = clampDockPosition(position.x, position.y)
  dictationDock.x = clamped.x
  dictationDock.y = clamped.y
  dictationDock.ready = true
}

function clampDictationDock() {
  if (!dictationDock.ready) return
  const clamped = clampDockPosition(dictationDock.x, dictationDock.y)
  dictationDock.x = clamped.x
  dictationDock.y = clamped.y
}

function onDictationDockPointerMove(event: PointerEvent) {
  if (!dictationDock.dragging || !host.value) return
  const rect = host.value.getBoundingClientRect()
  const next = clampDockPosition(event.clientX - rect.left, event.clientY - rect.top)
  if (Math.hypot(next.x - dictationDock.startX, next.y - dictationDock.startY) > 4) {
    dictationDock.moved = true
  }
  dictationDock.x = next.x
  dictationDock.y = next.y
}

function stopDictationDockDrag() {
  if (!dictationDock.dragging) return
  dictationDock.dragging = false
  saveDictationDock()
  window.removeEventListener('pointermove', onDictationDockPointerMove)
  window.removeEventListener('pointerup', stopDictationDockDrag)
}

function startDictationDockDrag(event: PointerEvent) {
  if (event.button !== 0 || !host.value) return
  const rect = host.value.getBoundingClientRect()
  dictationDock.dragging = true
  dictationDock.moved = false
  dictationDock.startX = event.clientX - rect.left
  dictationDock.startY = event.clientY - rect.top
  window.addEventListener('pointermove', onDictationDockPointerMove)
  window.addEventListener('pointerup', stopDictationDockDrag)
}

function onDictationDockClick(event: MouseEvent) {
  if (dictationDock.moved) {
    event.preventDefault()
    dictationDock.moved = false
    return
  }
  toggleDictation()
}

function speechRecognitionCtor(): SpeechRecognitionCtor | null {
  const speechWindow = window as Window & {
    SpeechRecognition?: SpeechRecognitionCtor
    webkitSpeechRecognition?: SpeechRecognitionCtor
  }
  return speechWindow.SpeechRecognition ?? speechWindow.webkitSpeechRecognition ?? null
}

function recognitionLang() {
  const lang = settings.speechLanguage.trim()
  if (lang === 'id') return 'id-ID'
  if (lang === 'en') return 'en-US'
  if (lang === 'ja') return 'ja-JP'
  if (lang === 'ko') return 'ko-KR'
  if (lang === 'zh') return 'zh-CN'
  return navigator.language || 'en-US'
}

function normalizeDevSpeech(text: string) {
  if (settings.speechProvider !== 'browser') return text

  return text
    .replace(/\b(frame work|frameworks|frimework|prem work|premwork)\b/gi, 'framework')
    .replace(/\b(repositori|repository|repo sitori|repo story)\b/gi, 'repository')
    .replace(/\b(pul request|pull reques|pull request|pool request|pur request)\b/gi, 'pull request')
    .replace(/\b(branch|brens|brance|brans)\b/gi, 'branch')
    .replace(/\b(komit|commit|komet)\b/gi, 'commit')
    .replace(/\b(merge|merj|marge)\b/gi, 'merge')
    .replace(/\b(push|pus)\b/gi, 'push')
    .replace(/\b(deploy|diploy|di ploy)\b/gi, 'deploy')
    .replace(/\b(front end|frontend)\b/gi, 'frontend')
    .replace(/\b(back end|backend)\b/gi, 'backend')
    .replace(/\b(open source|opensors|open sores)\b/gi, 'open source')
    .replace(/\b(type script|typescript)\b/gi, 'TypeScript')
    .replace(/\b(java script|javascript)\b/gi, 'JavaScript')
    .replace(/\b(react js|react)\b/gi, 'React')
    .replace(/\b(next js|next\.?js)\b/gi, 'Next.js')
    .replace(/\b(nuxt js|nuxt\.?js)\b/gi, 'Nuxt')
    .replace(/\b(vue js|view js|vue)\b/gi, 'Vue')
    .replace(/\b(node js|node\.?js)\b/gi, 'Node.js')
    .replace(/\b(tailwind|tail win|tailwind css)\b/gi, 'Tailwind CSS')
    .replace(/\b(git hub|github)\b/gi, 'GitHub')
    .replace(/\b(git lab|gitlab)\b/gi, 'GitLab')
}

async function writePastedText(text: string) {
  const normalized = normalizeDevSpeech(text).trim()
  if (!normalized) return
  const bracketed = term?.modes.bracketedPasteMode
  const payload = bracketed ? `\x1b[200~${normalized}\x1b[201~` : normalized
  await invoke('pty_write', { id: props.sessionId, data: payload })
}

function clearSilenceTimer() {
  clearTimeout(silenceTimer)
  silenceTimer = undefined
}

function flushPendingTranscript() {
  const text = pendingTranscript.trim()
  if (!text || committedUtterance) return
  committedUtterance = true
  stoppingDictation = true
  pendingTranscript = ''
  transcriptBuffer = ''
  dictationPreview.value = ''
  clearSilenceTimer()
  writePastedText(text).catch(() => {})
  recognition?.stop()
}

function scheduleSilenceCommit() {
  clearSilenceTimer()
  silenceTimer = setTimeout(flushPendingTranscript, SILENCE_COMMIT_MS)
}

function stopDictation() {
  if (settings.speechProvider === 'groq') {
    stopGroqRecording(true)
    return
  }
  stoppingDictation = true
  flushPendingTranscript()
  recognition?.stop()
}

function groqLanguageHint() {
  const lang = settings.speechLanguage.trim().toLowerCase().replace('_', '-')
  if (!lang || lang === 'auto') return null
  if (lang.startsWith('id')) return 'id'
  if (lang.startsWith('en')) return 'en'
  if (lang.startsWith('ja')) return 'ja'
  if (lang.startsWith('ko')) return 'ko'
  if (lang.startsWith('zh')) return 'zh'
  return null
}

function cleanupGroqRecording() {
  cancelAnimationFrame(silenceFrame)
  silenceFrame = 0
  mediaStream?.getTracks().forEach(track => track.stop())
  mediaStream = undefined
  audioContext?.close().catch(() => {})
  audioContext = undefined
  analyser = undefined
  frequencyData = undefined
  speechStartedAt = null
  lastSpeechAt = null
  groqSpectrum.value = Array.from({ length: GROQ_SPECTRUM_BARS }, () => 0.18)
}

async function transcribeGroqAudio(blob: Blob) {
  if (!blob.size) {
    dictating.value = false
    dictationPreview.value = ''
    return
  }
  transcribing.value = true
  dictationPreview.value = 'Transcribing…'
  try {
    const bytes = Array.from(new Uint8Array(await blob.arrayBuffer()))
    const text = await invoke<string>('speech_groq_transcribe', {
      audio: bytes,
      mime: blob.type || 'audio/webm',
      language: groqLanguageHint()
    })
    await writePastedText(text)
  } catch (error) {
    toast.add({
      title: 'Groq transcription failed',
      description: String(error),
      icon: 'i-lucide-mic-off',
      color: 'error'
    })
  } finally {
    transcribing.value = false
    dictating.value = false
    dictationPreview.value = ''
  }
}

function stopGroqRecording(manual = false) {
  manualGroqStop = manual
  if (mediaRecorder && mediaRecorder.state !== 'inactive') {
    mediaRecorder.stop()
    return
  }
  cleanupGroqRecording()
  dictating.value = false
  dictationPreview.value = ''
  manualGroqStop = false
}

function updateGroqSpectrum(data: Uint8Array<ArrayBuffer>) {
  const samplesPerBar = Math.max(1, Math.floor(data.length / GROQ_SPECTRUM_BARS))
  const levels = Array.from({ length: GROQ_SPECTRUM_BARS }, (_, bar) => {
    let sum = 0
    const start = bar * samplesPerBar
    const end = Math.min(data.length, start + samplesPerBar)
    for (let index = start; index < end; index += 1) {
      const centered = Math.abs((data[index] ?? 128) - 128)
      sum += centered * centered
    }
    const rms = Math.sqrt(sum / Math.max(1, end - start))
    const idle = 0.18 + Math.sin(Date.now() / 180 + bar * 0.7) * 0.045
    return Math.max(idle, Math.min(1, rms / 82))
  })
  groqSpectrum.value = levels
}

function watchGroqSilence() {
  if (!analyser || !dictating.value) return
  const data = new Uint8Array(analyser.fftSize)
  analyser.getByteTimeDomainData(data)
  frequencyData ??= new Uint8Array(analyser.frequencyBinCount)
  analyser.getByteFrequencyData(frequencyData)
  updateGroqSpectrum(frequencyData)
  let sum = 0
  for (const value of data) {
    const centered = value - 128
    sum += centered * centered
  }
  const volume = Math.sqrt(sum / data.length) / 128
  const now = Date.now()

  if (volume > GROQ_VOICE_THRESHOLD) {
    speechStartedAt ??= now
    lastSpeechAt = now
    clearSilenceTimer()
    dictationPreview.value = ''
  } else if (speechStartedAt && lastSpeechAt && now - lastSpeechAt > 300 && !silenceTimer) {
    silenceTimer = setTimeout(() => stopGroqRecording(false), SILENCE_COMMIT_MS)
  }

  silenceFrame = requestAnimationFrame(watchGroqSilence)
}

function preferredRecordingMime() {
  return [
    'audio/webm;codecs=opus',
    'audio/webm',
    'audio/mp4',
    'audio/ogg;codecs=opus',
    'audio/ogg'
  ].find(type => MediaRecorder.isTypeSupported(type)) ?? ''
}

async function startGroqRecording() {
  if (!navigator.mediaDevices?.getUserMedia || typeof MediaRecorder === 'undefined') {
    toast.add({
      title: 'Microphone unavailable',
      description: 'This webview cannot record audio.',
      icon: 'i-lucide-mic-off',
      color: 'warning'
    })
    return
  }

  try {
    recordedChunks = []
    mediaStream = await navigator.mediaDevices.getUserMedia({ audio: true })
    audioContext = new AudioContext()
    analyser = audioContext.createAnalyser()
    analyser.fftSize = 2048
    analyser.smoothingTimeConstant = 0.72
    frequencyData = new Uint8Array(analyser.frequencyBinCount)
    audioContext.createMediaStreamSource(mediaStream).connect(analyser)
    const mime = preferredRecordingMime()
    mediaRecorder = mime
      ? new MediaRecorder(mediaStream, { mimeType: mime })
      : new MediaRecorder(mediaStream)
    mediaRecorder.ondataavailable = (event) => {
      if (event.data.size) recordedChunks.push(event.data)
    }
    mediaRecorder.onstop = () => {
      const speechMs = speechStartedAt && lastSpeechAt ? lastSpeechAt - speechStartedAt : 0
      const shouldTranscribe = manualGroqStop || Boolean(speechStartedAt && speechMs >= GROQ_MIN_AUTO_SPEECH_MS)
      const blob = new Blob(recordedChunks, { type: mediaRecorder?.mimeType || mime })
      cleanupGroqRecording()
      if (shouldTranscribe) {
        transcribeGroqAudio(blob)
      } else {
        transcribing.value = false
        dictating.value = false
        dictationPreview.value = ''
      }
      mediaRecorder = undefined
      manualGroqStop = false
    }
    dictating.value = true
    dictationPreview.value = ''
    speechStartedAt = null
    lastSpeechAt = null
    manualGroqStop = false
    mediaRecorder.start(250)
    watchGroqSilence()
  } catch (error) {
    cleanupGroqRecording()
    dictating.value = false
    dictationPreview.value = ''
    toast.add({
      title: 'Microphone failed',
      description: String(error),
      icon: 'i-lucide-mic-off',
      color: 'error'
    })
  }
}

function createRecognition(): SpeechRecognitionLike | null {
  const Recognition = speechRecognitionCtor()
  if (!Recognition) {
    toast.add({
      title: 'Speech-to-text unavailable',
      description: 'This webview does not expose browser speech recognition.',
      icon: 'i-lucide-mic-off',
      color: 'warning'
    })
    return null
  }

  const next = new Recognition()
  next.lang = recognitionLang()
  next.interimResults = true
  next.continuous = true
  next.maxAlternatives = 1
  next.onstart = () => {
    dictating.value = true
  }
  next.onend = () => {
    recognition = undefined
    if (!dictating.value || stoppingDictation || committedUtterance) {
      dictating.value = false
      dictationPreview.value = ''
      pendingTranscript = ''
      transcriptBuffer = ''
      clearSilenceTimer()
      return
    }
    window.setTimeout(() => {
      if (dictating.value && !recognition && !stoppingDictation && !committedUtterance) {
        startRecognition()
      }
    }, 180)
  }
  next.onerror = (event) => {
    if (event.error === 'no-speech' && dictating.value && !stoppingDictation) {
      recognition = undefined
      startRecognition()
      return
    }
    dictating.value = false
    dictationPreview.value = ''
    pendingTranscript = ''
    transcriptBuffer = ''
    clearSilenceTimer()
    stoppingDictation = false
    recognition = undefined
    toast.add({
      title: 'Dictation failed',
      description: event.error || 'Speech recognition stopped unexpectedly.',
      icon: 'i-lucide-mic-off',
      color: 'error'
    })
  }
  next.onresult = (event) => {
    if (committedUtterance) return
    let finalText = ''
    let interimText = ''
    for (let index = event.resultIndex; index < event.results.length; index += 1) {
      const result = event.results[index]
      const transcript = result?.[0]?.transcript ?? ''
      if (result?.isFinal) finalText += transcript
      else interimText += transcript
    }
    if (finalText.trim()) {
      transcriptBuffer = `${transcriptBuffer} ${finalText}`.trim()
    }
    pendingTranscript = normalizeDevSpeech(`${transcriptBuffer} ${interimText}`.trim())
    dictationPreview.value = pendingTranscript
    if (pendingTranscript) {
      scheduleSilenceCommit()
    }
  }

  return next
}

function startRecognition() {
  const next = createRecognition()
  if (!next) return
  recognition = next
  try {
    recognition.start()
  } catch (error) {
    recognition = undefined
    dictating.value = false
    pendingTranscript = ''
    transcriptBuffer = ''
    clearSilenceTimer()
    stoppingDictation = false
    toast.add({
      title: 'Dictation failed',
      description: String(error),
      icon: 'i-lucide-mic-off',
      color: 'error'
    })
  }
}

function toggleDictation() {
  if (transcribing.value) return
  if (dictating.value) {
    stopDictation()
    return
  }

  if (settings.speechProvider === 'groq') {
    startGroqRecording()
    return
  }

  recognition?.abort()
  dictationPreview.value = ''
  pendingTranscript = ''
  transcriptBuffer = ''
  committedUtterance = false
  stoppingDictation = false
  clearSilenceTimer()
  startRecognition()
}

onBeforeUnmount(() => {
  stoppingDictation = true
  clearSilenceTimer()
  cleanupGroqRecording()
  window.removeEventListener('pointermove', onDictationDockPointerMove)
  window.removeEventListener('pointerup', stopDictationDockDrag)
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

defineExpose({ focus, findNext, findPrevious, clearSearch, toggleDictation })
</script>

<template>
  <!-- padding on the wrapper so FitAddon measures the host correctly -->
  <div
    ref="host"
    class="relative h-full w-full bg-[#0e0e10] px-3 py-2"
  >
    <div
      ref="el"
      class="h-full w-full"
    />
    <div
      v-if="dictating || transcribing"
      class="absolute z-30"
      :style="dictationDockStyle"
    >
      <button
        class="group flex h-12 touch-none select-none items-center justify-center gap-2 rounded-2xl border shadow-2xl ring-1 ring-black/40 backdrop-blur-md transition-all"
        :class="dictationDockButtonClass"
        title="Dictate text"
        aria-label="Dictate text"
        @pointerdown.stop="startDictationDockDrag"
        @click.stop="onDictationDockClick"
      >
        <UIcon
          :name="transcribing ? 'i-lucide-loader-circle' : (dictating ? 'i-lucide-square' : 'i-lucide-mic')"
          class="size-5 shrink-0"
          :class="[
            dictating ? 'text-blue-300' : 'group-hover:text-blue-300',
            transcribing ? 'animate-spin' : ''
          ]"
        />
        <span
          v-if="!dictating"
          class="text-[12px] font-medium"
        >
          Dictate
        </span>
        <div
          v-if="dictating && settings.speechProvider === 'groq'"
          class="flex h-7 w-44 shrink-0 items-center justify-center gap-1"
          aria-hidden="true"
        >
          <span
            v-for="(level, index) in groqSpectrum"
            :key="index"
            class="h-5 w-1 rounded-full bg-blue-300/85 transition-transform duration-75"
            :class="transcribing ? 'animate-pulse' : ''"
            :style="{ transform: `scaleY(${level})` }"
          />
        </div>
        <span
          v-if="dictating && settings.speechProvider === 'browser'"
          class="shrink-0 text-[12px] font-medium text-blue-100"
        >
          Listening
        </span>
        <span
          v-if="dictating && settings.speechProvider === 'browser'"
          class="h-5 w-px shrink-0 bg-white/15"
        />
        <span
          v-if="dictating && settings.speechProvider === 'browser'"
          class="min-w-0 truncate text-[11px] font-mono text-blue-100/80"
        >
          {{ dictationPreview || 'Listening…' }}
        </span>
      </button>
    </div>
  </div>
</template>
