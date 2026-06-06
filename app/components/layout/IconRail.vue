<script setup lang="ts">
const ui = useUiStore()

const views = [
  { id: 'terminal' as const, icon: 'i-lucide-terminal', label: 'Terminal (⌘1)' },
  { id: 'changes' as const, icon: 'i-lucide-git-branch', label: 'File Changes (⌘2)' }
]
</script>

<template>
  <aside class="flex flex-col items-center w-12 shrink-0 border-r border-default bg-muted/60">
    <!-- breathing room under the macOS traffic lights; draggable -->
    <div
      data-tauri-drag-region
      class="h-12 w-full shrink-0"
    />

    <!-- logo glyph -->
    <div class="flex items-center justify-center size-8 mb-3 rounded-lg bg-elevated border border-default">
      <UIcon
        name="i-lucide-waves"
        class="size-4 text-toned"
      />
    </div>

    <!-- view switcher -->
    <nav class="flex flex-col items-center gap-1">
      <UTooltip
        v-for="view in views"
        :key="view.id"
        :text="view.label"
        :content="{ side: 'right' }"
      >
        <button
          class="flex items-center justify-center size-8 rounded-lg transition-colors"
          :class="ui.view === view.id
            ? 'bg-elevated text-highlighted ring-1 ring-(--ui-border-accented)'
            : 'text-dimmed hover:text-toned hover:bg-elevated/50'"
          :aria-label="view.label"
          @click="ui.view = view.id"
        >
          <UIcon
            :name="view.icon"
            class="size-4"
          />
        </button>
      </UTooltip>
    </nav>

    <div
      data-tauri-drag-region
      class="flex-1 w-full"
    />

    <!-- account -->
    <div class="pb-3">
      <GithubMenu />
    </div>
  </aside>
</template>
