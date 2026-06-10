<script setup lang="ts">
import { listen, type UnlistenFn } from '@tauri-apps/api/event'

useHead({
  title: 'Mainbound',
  htmlAttrs: { lang: 'en' }
})

const ui = useUiStore()
const terminals = useTerminalsStore()
const workspaces = useWorkspacesStore()
const notifications = useNotificationsStore()
const updater = useUpdaterStore()
const settingsStore = useSettingsStore()
const { isMac } = usePlatform()

let unlistenMenu: UnlistenFn | undefined

// Single source of truth for menu + keyboard actions
function dispatchAction(action: string) {
  switch (action) {
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
    case 'zoom-in':
    case 'zoom-out':
    case 'zoom-reset':
      void applyZoom(action)
      break
  }
}

// Zoom is handled natively on macOS (menu); on Windows/Linux frameless
// windows have no menu bar, so drive it from JS via the webview API.
async function applyZoom(action: string) {
  const { getCurrentWebview } = await import('@tauri-apps/api/webview')
  const wv = getCurrentWebview()
  const cur = ui.zoom
  const next = action === 'zoom-in'
    ? Math.min(3, cur + 0.1)
    : action === 'zoom-out'
      ? Math.max(0.5, cur - 0.1)
      : 1
  ui.zoom = Math.round(next * 100) / 100
  await wv.setZoom(ui.zoom).catch(() => {})
}

// Cross-platform keyboard shortcuts. Native menu accelerators only fire
// reliably on macOS (and not at all on frameless Windows/Linux windows
// that have no menu bar), so handle the keys directly there.
function onKeydown(e: KeyboardEvent) {
  const mod = isMac.value ? e.metaKey : e.ctrlKey
  if (!mod) return

  // ignore when typing in an input/textarea (except our explicit combos)
  const key = e.key.toLowerCase()
  const map: Record<string, string> = {
    't': 'new-session',
    'w': 'close-session',
    '1': 'view-terminal',
    '2': 'view-changes',
    'k': 'command-palette',
    'f': 'find',
    '=': 'zoom-in',
    '+': 'zoom-in',
    '-': 'zoom-out',
    '_': 'zoom-out',
    '0': 'zoom-reset'
  }
  const action = key === 'd'
    ? (e.shiftKey ? 'split-down' : 'split-right')
    : map[key] ?? null
  if (action) {
    e.preventDefault()
    dispatchAction(action)
  }
}

onMounted(async () => {
  notifications.init()
  updater.init()
  await settingsStore.load()
  if (settingsStore.settings.autoUpdateCheck) {
    setTimeout(() => updater.check(false), 5_000)
  }

  // native menu (macOS, and Windows/Linux when a menu bar exists)
  unlistenMenu = await listen<string>('menu://action', ({ payload }) => dispatchAction(payload))

  // JS keyboard fallback — primary path on Windows/Linux
  if (!isMac.value) {
    window.addEventListener('keydown', onKeydown, true)
    // restore persisted zoom on these platforms (macOS does it natively)
    if (ui.zoom !== 1) {
      const { getCurrentWebview } = await import('@tauri-apps/api/webview')
      await getCurrentWebview().setZoom(ui.zoom).catch(() => {})
    }
  }
})

onBeforeUnmount(() => {
  unlistenMenu?.()
  window.removeEventListener('keydown', onKeydown, true)
})
</script>

<template>
  <UApp>
    <NuxtPage />
    <UpdateModal />
    <CommandPalette />
    <SettingsModal />
  </UApp>
</template>
