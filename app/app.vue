<script setup lang="ts">
import { listen, type UnlistenFn } from '@tauri-apps/api/event'

useHead({
  title: 'Mainbound',
  htmlAttrs: { lang: 'en' }
})

// Native menu actions (⌘T / ⌘D / ⇧⌘D / ⌘W / ⌘1 / ⌘2)
const ui = useUiStore()
const terminals = useTerminalsStore()
const workspaces = useWorkspacesStore()
const notifications = useNotificationsStore()
const updater = useUpdaterStore()

let unlistenMenu: UnlistenFn | undefined

const settingsStore = useSettingsStore()

onMounted(async () => {
  notifications.init()
  updater.init()
  await settingsStore.load()
  if (settingsStore.settings.autoUpdateCheck) {
    // silent auto-check shortly after launch
    setTimeout(() => updater.check(false), 5_000)
  }
  unlistenMenu = await listen<string>('menu://action', ({ payload }) => {
    switch (payload) {
      case 'new-session':
        ui.view = 'terminal'
        terminals.create(workspaces.active?.path ?? null, workspaces.active?.name)
        break
      case 'split-right':
        if (ui.view === 'terminal') terminals.split('row')
        break
      case 'split-down':
        if (ui.view === 'terminal') terminals.split('column')
        break
      case 'close-session':
        if (ui.view === 'terminal' && terminals.focusedSessionId) {
          terminals.kill(terminals.focusedSessionId)
        }
        break
      case 'view-terminal':
        ui.view = 'terminal'
        break
      case 'view-changes':
        ui.view = 'changes'
        break
      case 'check-updates':
        updater.check(true)
        break
      case 'command-palette':
        ui.paletteOpen = !ui.paletteOpen
        break
      case 'find':
        if (ui.view === 'terminal') ui.terminalSearchOpen = true
        break
    }
  })
})

onBeforeUnmount(() => unlistenMenu?.())
</script>

<template>
  <UApp>
    <NuxtPage />
    <UpdateModal />
    <CommandPalette />
    <SettingsModal />
  </UApp>
</template>
