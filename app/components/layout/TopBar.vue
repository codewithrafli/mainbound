<script setup lang="ts">
const ui = useUiStore()
const terminals = useTerminalsStore()
const workspaces = useWorkspacesStore()
const { cmdKey } = usePlatform()

const views = computed(() => [
  { id: 'terminal' as const, label: 'Terminal', kbd: `${cmdKey.value}1` },
  { id: 'changes' as const, label: 'File Changes', kbd: `${cmdKey.value}2` }
])

function newSession() {
  ui.view = 'terminal'
  terminals.create(workspaces.active?.path ?? null, workspaces.active?.name)
}
</script>

<template>
  <header
    data-tauri-drag-region
    class="relative flex items-center h-12 shrink-0 gap-2 px-3 border-b border-default select-none"
  >
    <LayoutWorkspaceSwitcher />

    <span
      data-tauri-drag-region
      class="flex-1 h-full"
    />

    <!-- centered view toggle with shortcut hints -->
    <div class="absolute left-1/2 -translate-x-1/2 flex items-center gap-0.5 rounded-full border border-default bg-muted/60 p-0.5">
      <button
        v-for="view in views"
        :key="view.id"
        class="flex items-center gap-1.5 px-3 py-1 rounded-full text-xs transition-colors"
        :class="ui.view === view.id
          ? 'bg-elevated text-highlighted ring-1 ring-(--ui-border-accented)'
          : 'text-muted hover:text-toned'"
        @click="ui.view = view.id"
      >
        {{ view.label }}
        <kbd class="text-[9px] font-mono text-dimmed">{{ view.kbd }}</kbd>
      </button>
    </div>

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
