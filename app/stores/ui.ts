export type AppView = 'terminal' | 'changes'

export const useUiStore = defineStore('ui', () => {
  const view = ref<AppView>('terminal')
  const paletteOpen = ref(false)
  const terminalSearchOpen = ref(false)
  const onboardingSkipped = ref(false)

  return { view, paletteOpen, terminalSearchOpen, onboardingSkipped }
})
