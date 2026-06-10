<script setup lang="ts">
// Custom window controls for frameless Windows (and Linux) windows.
// macOS keeps its native traffic lights, so this renders only off-mac.
import { getCurrentWindow } from '@tauri-apps/api/window'

const isMaximized = ref(false)
let win: ReturnType<typeof getCurrentWindow> | undefined
let unlisten: (() => void) | undefined

onMounted(async () => {
  win = getCurrentWindow()
  isMaximized.value = await win.isMaximized().catch(() => false)
  unlisten = await win.onResized(async () => {
    isMaximized.value = await win!.isMaximized().catch(() => false)
  })
})
onBeforeUnmount(() => unlisten?.())

function minimize() {
  win?.minimize()
}
function toggleMaximize() {
  win?.toggleMaximize()
}
function close() {
  // triggers onCloseRequested → session flush → destroy
  win?.close()
}
</script>

<template>
  <div class="flex items-center h-full -mr-3">
    <button
      class="flex items-center justify-center w-11 h-full text-dimmed hover:text-toned hover:bg-elevated/60 transition-colors"
      aria-label="Minimize"
      @click="minimize"
    >
      <UIcon
        name="i-lucide-minus"
        class="size-4"
      />
    </button>
    <button
      class="flex items-center justify-center w-11 h-full text-dimmed hover:text-toned hover:bg-elevated/60 transition-colors"
      :aria-label="isMaximized ? 'Restore' : 'Maximize'"
      @click="toggleMaximize"
    >
      <UIcon
        :name="isMaximized ? 'i-lucide-copy' : 'i-lucide-square'"
        class="size-3.5"
      />
    </button>
    <button
      class="flex items-center justify-center w-11 h-full text-dimmed hover:text-white hover:bg-red-600 transition-colors"
      aria-label="Close"
      @click="close"
    >
      <UIcon
        name="i-lucide-x"
        class="size-4"
      />
    </button>
  </div>
</template>
