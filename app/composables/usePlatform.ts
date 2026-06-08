/** Detect the host OS at runtime via the Tauri OS plugin or navigator. */
export function usePlatform() {
  const isMac = computed(() => {
    // navigator.platform is deprecated but works fine for this use case
    if (typeof navigator === 'undefined') return true
    return navigator.platform.toUpperCase().includes('MAC')
      || navigator.userAgent.includes('Macintosh')
  })

  const isWindows = computed(() => {
    if (typeof navigator === 'undefined') return false
    return navigator.platform.toUpperCase().includes('WIN')
  })

  /** Show ⌘ on Mac, Ctrl on Windows/Linux */
  const cmdKey = computed(() => isMac.value ? '⌘' : 'Ctrl')

  /** Modifier key label for display (⌥ on Mac, Alt on Windows) */
  const altKey = computed(() => isMac.value ? '⌥' : 'Alt')

  return { isMac, isWindows, cmdKey, altKey }
}
