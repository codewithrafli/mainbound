<script setup lang="ts">
const ui = useUiStore()
const terminals = useTerminalsStore()
const workspaces = useWorkspacesStore()

function newSession() {
  ui.view = 'terminal'
  terminals.create(workspaces.active?.path ?? null, workspaces.active?.name)
}
</script>

<template>
  <header
    data-tauri-drag-region
    class="flex items-center h-12 shrink-0 gap-2 px-3 border-b border-default select-none"
  >
    <LayoutWorkspaceSwitcher />
    <span class="text-[11px] text-dimmed">
      {{ ui.view === 'terminal' ? 'Terminal' : 'File Changes' }}
    </span>

    <span
      data-tauri-drag-region
      class="flex-1 h-full"
    />

    <!-- white primary CTA, console-style -->
    <UButton
      label="New Session"
      icon="i-lucide-plus"
      color="neutral"
      variant="solid"
      size="xs"
      @click="newSession"
    />
  </header>
</template>
