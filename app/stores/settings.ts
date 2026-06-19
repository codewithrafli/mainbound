import { invoke } from '@tauri-apps/api/core'

export interface AppSettings {
  /** terminal font size in px */
  fontSize: number
  /** custom shell path; empty = $SHELL */
  shell: string
  /** AI provider used for commit and PR generation */
  aiProvider: 'claude' | 'codex'
  /** seconds of sustained output before "command finished" fires */
  notifMinBurst: number
  /** silent update check on launch */
  autoUpdateCheck: boolean
  /** auto-create draft PR when pushing a new branch */
  autoDraftPr: boolean
  /** speech-to-text backend */
  speechProvider: 'browser' | 'groq'
  /** speech recognition/transcription language hint */
  speechLanguage: string
  /** non-secret local flag; avoids probing macOS Keychain for UI status */
  speechGroqKeyConfigured: boolean
}

const DEFAULTS: AppSettings = {
  fontSize: 12.5,
  shell: '',
  aiProvider: 'claude',
  notifMinBurst: 10,
  autoUpdateCheck: true,
  autoDraftPr: false,
  speechProvider: 'groq',
  speechLanguage: 'auto',
  speechGroqKeyConfigured: false
}

function normalizeSpeechLanguage(value: string | undefined) {
  const language = value?.trim().toLowerCase().replace('_', '-') || 'auto'
  if (language === 'auto') return 'auto'
  if (language.startsWith('id')) return 'id'
  if (language.startsWith('en')) return 'en'
  if (language.startsWith('ja')) return 'ja'
  if (language.startsWith('ko')) return 'ko'
  if (language.startsWith('zh')) return 'zh'
  return 'auto'
}

export const useSettingsStore = defineStore('settings', () => {
  const settings = reactive<AppSettings>({ ...DEFAULTS })
  const loaded = ref(false)
  const modalOpen = ref(false)

  async function load() {
    if (loaded.value) return
    loaded.value = true
    const saved = await invoke<Partial<AppSettings> | null>('settings_load').catch(() => null)
    if (saved && typeof saved === 'object') {
      Object.assign(settings, { ...DEFAULTS, ...saved })
      settings.speechLanguage = normalizeSpeechLanguage(settings.speechLanguage)
    }
  }

  let saveTimer: ReturnType<typeof setTimeout> | undefined
  watch(settings, () => {
    if (!loaded.value) return
    clearTimeout(saveTimer)
    saveTimer = setTimeout(() => {
      invoke('settings_save', { data: { ...settings } }).catch(() => {})
    }, 500)
  })

  return { settings, loaded, modalOpen, load }
})
