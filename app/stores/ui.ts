export type AppView = 'terminal' | 'changes'

export const useUiStore = defineStore('ui', () => {
  const view = ref<AppView>('terminal')

  return { view }
})
