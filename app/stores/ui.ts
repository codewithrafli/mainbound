export type AppView = 'terminal' | 'changes'

export const useUiStore = defineStore('ui', () => {
  const view = ref<AppView>('terminal')
  const paletteOpen = ref(false)
  const terminalSearchOpen = ref(false)
  const onboardingSkipped = ref(false)

  // collapsible side panels (persisted to localStorage)
  const leftSidebarOpen = ref(true)
  const rightPanelOpen = ref(true)
  // webview zoom (driven from JS on Windows/Linux; macOS uses native menu)
  const zoom = ref(1)

  function toggleLeftSidebar() {
    leftSidebarOpen.value = !leftSidebarOpen.value
  }
  function toggleRightPanel() {
    rightPanelOpen.value = !rightPanelOpen.value
  }

  if (import.meta.client) {
    const saved = localStorage.getItem('mb-panels')
    if (saved) {
      try {
        const { left, right, zoom: z } = JSON.parse(saved)
        leftSidebarOpen.value = left ?? true
        rightPanelOpen.value = right ?? true
        if (typeof z === 'number') zoom.value = z
      } catch { /* ignore */ }
    }
    watch([leftSidebarOpen, rightPanelOpen, zoom], ([left, right, z]) => {
      localStorage.setItem('mb-panels', JSON.stringify({ left, right, zoom: z }))
    })
  }

  return {
    view, paletteOpen, terminalSearchOpen, onboardingSkipped,
    leftSidebarOpen, rightPanelOpen, zoom, toggleLeftSidebar, toggleRightPanel
  }
})
