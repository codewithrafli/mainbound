export type AppView = 'terminal' | 'changes'

export const useUiStore = defineStore('ui', () => {
  const view = ref<AppView>('terminal')
  const paletteOpen = ref(false)
  const terminalSearchOpen = ref(false)
  const onboardingSkipped = ref(false)

  // collapsible side panels (persisted to localStorage)
  const leftSidebarOpen = ref(true)
  const rightPanelOpen = ref(true)

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
        const { left, right } = JSON.parse(saved)
        leftSidebarOpen.value = left ?? true
        rightPanelOpen.value = right ?? true
      } catch { /* ignore */ }
    }
    watch([leftSidebarOpen, rightPanelOpen], ([left, right]) => {
      localStorage.setItem('mb-panels', JSON.stringify({ left, right }))
    })
  }

  return {
    view, paletteOpen, terminalSearchOpen, onboardingSkipped,
    leftSidebarOpen, rightPanelOpen, toggleLeftSidebar, toggleRightPanel
  }
})
