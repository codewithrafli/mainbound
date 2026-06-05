<script setup lang="ts">
import type { TabsItem } from '@nuxt/ui'

const ui = useUiStore()

const viewTabs: TabsItem[] = [
  { label: 'Terminal', value: 'terminal' },
  { label: 'File Changes', value: 'changes' }
]
</script>

<template>
  <header
    data-tauri-drag-region
    class="relative flex items-center h-11 shrink-0 px-3 border-b border-default bg-muted"
  >
    <!-- left: workspace switcher, offset for macOS traffic lights -->
    <div class="flex items-center gap-2 pl-[72px]">
      <LayoutWorkspaceSwitcher />
    </div>

    <!-- center: view toggle -->
    <div class="absolute left-1/2 -translate-x-1/2">
      <UTabs
        v-model="ui.view"
        :items="viewTabs"
        :content="false"
        color="neutral"
        size="xs"
      />
    </div>

    <!-- right: actions -->
    <div class="ml-auto flex items-center gap-1.5">
      <GithubMenu />
    </div>
  </header>
</template>
