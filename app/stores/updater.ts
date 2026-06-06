export const useUpdaterStore = defineStore('updater', () => {
  const currentVersion = ref('')
  const available = ref(false)
  const newVersion = ref('')
  const changelog = ref('')
  const releaseDate = ref('')
  const checking = ref(false)
  const installing = ref(false)
  const progress = ref(0)
  const error = ref<string | null>(null)
  const modalOpen = ref(false)
  /** "no update" feedback only for manual checks */
  const upToDate = ref(false)

  let updateHandle: import('@tauri-apps/plugin-updater').Update | null = null

  async function init() {
    const { getVersion } = await import('@tauri-apps/api/app')
    currentVersion.value = await getVersion().catch(() => '')
  }

  async function check(manual = false) {
    if (checking.value) return
    checking.value = true
    error.value = null
    upToDate.value = false
    try {
      const { check: checkUpdate } = await import('@tauri-apps/plugin-updater')
      const update = await checkUpdate()
      if (update) {
        updateHandle = update
        available.value = true
        newVersion.value = update.version
        changelog.value = update.body ?? ''
        releaseDate.value = update.date ?? ''
        modalOpen.value = true
      } else if (manual) {
        upToDate.value = true
        modalOpen.value = true
      }
    } catch (e) {
      // dev mode / offline — only surface on manual checks
      if (manual) {
        error.value = String(e)
        modalOpen.value = true
      }
    } finally {
      checking.value = false
    }
  }

  async function installAndRelaunch() {
    if (!updateHandle || installing.value) return
    installing.value = true
    error.value = null
    let total = 0
    let received = 0
    try {
      await updateHandle.downloadAndInstall((event) => {
        if (event.event === 'Started') {
          total = event.data.contentLength ?? 0
        } else if (event.event === 'Progress') {
          received += event.data.chunkLength ?? 0
          if (total) progress.value = Math.round((received / total) * 100)
        } else if (event.event === 'Finished') {
          progress.value = 100
        }
      })
      const { relaunch } = await import('@tauri-apps/plugin-process')
      await relaunch()
    } catch (e) {
      error.value = String(e)
      installing.value = false
    }
  }

  return {
    currentVersion, available, newVersion, changelog, releaseDate,
    checking, installing, progress, error, modalOpen, upToDate,
    init, check, installAndRelaunch
  }
})
