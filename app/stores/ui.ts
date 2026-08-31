export type AppView = 'terminal' | 'changes' | 'explorer'

export const useUiStore = defineStore('ui', () => {
  const view = ref<AppView>('terminal')
  const paletteOpen = ref(false)
  const terminalSearchOpen = ref(false)
  const onboardingSkipped = ref(false)

  // collapsible side panels (persisted to localStorage)
  const terminalSidebarOpen = ref(true)
  const changesSidebarOpen = ref(true)
  const explorerSidebarOpen = ref(true)
  const rightPanelOpen = ref(true)
  // webview zoom (driven from JS on Windows/Linux; macOS uses native menu)
  const zoom = ref(1)

  const leftSidebarOpen = computed({
    get: () => {
      if (view.value === 'changes') return changesSidebarOpen.value
      if (view.value === 'explorer') return explorerSidebarOpen.value
      return terminalSidebarOpen.value
    },
    set: (open: boolean) => {
      if (view.value === 'changes') changesSidebarOpen.value = open
      else if (view.value === 'explorer') explorerSidebarOpen.value = open
      else terminalSidebarOpen.value = open
    }
  })

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
        const {
          left,
          terminalLeft,
          changesLeft,
          explorerLeft,
          right,
          zoom: z
        } = JSON.parse(saved)
        terminalSidebarOpen.value = terminalLeft ?? left ?? true
        changesSidebarOpen.value = changesLeft ?? left ?? true
        explorerSidebarOpen.value = explorerLeft ?? true
        rightPanelOpen.value = right ?? true
        if (typeof z === 'number') zoom.value = z
      } catch { /* ignore */ }
    }
    watch([terminalSidebarOpen, changesSidebarOpen, explorerSidebarOpen, rightPanelOpen, zoom], ([terminalLeft, changesLeft, explorerLeft, right, z]) => {
      localStorage.setItem('mb-panels', JSON.stringify({
        terminalLeft,
        changesLeft,
        explorerLeft,
        right,
        zoom: z
      }))
    })
  }

  return {
    view, paletteOpen, terminalSearchOpen, onboardingSkipped,
    leftSidebarOpen, terminalSidebarOpen, changesSidebarOpen, explorerSidebarOpen,
    rightPanelOpen, zoom, toggleLeftSidebar, toggleRightPanel
  }
})
