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
}

const DEFAULTS: AppSettings = {
  fontSize: 12.5,
  shell: '',
  aiProvider: 'claude',
  notifMinBurst: 10,
  autoUpdateCheck: true
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
