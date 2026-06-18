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
  /** BCP-47 speech recognition language; empty = browser default */
  speechLanguage: string
}

const DEFAULTS: AppSettings = {
  fontSize: 12.5,
  shell: '',
  aiProvider: 'claude',
  notifMinBurst: 10,
  autoUpdateCheck: true,
  autoDraftPr: false,
  speechProvider: 'groq',
  speechLanguage: 'auto'
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
      if (!settings.speechLanguage) settings.speechLanguage = 'auto'
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
