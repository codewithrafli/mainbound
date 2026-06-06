<script setup lang="ts">
import type { DropdownMenuItem } from '@nuxt/ui'

const workspaces = useWorkspacesStore()

const items = computed<DropdownMenuItem[][]>(() => [
  workspaces.list.map(w => ({
    label: w.name,
    type: 'checkbox' as const,
    checked: w.id === workspaces.activeId,
    onSelect: () => workspaces.setActive(w.id)
  })),
  [{
    label: 'Add Workspace…',
    icon: 'i-lucide-folder-plus',
    onSelect: () => workspaces.add()
  }]
])
</script>

<template>
  <UDropdownMenu
    :items="items"
    :content="{ align: 'start' }"
    :ui="{ content: 'w-56' }"
  >
    <!-- plain text + chevron, like the reference's context switcher -->
    <button class="flex items-center gap-1.5 px-2 py-1 rounded-lg text-sm font-medium text-highlighted hover:bg-elevated/60 transition-colors">
      <UIcon
        name="i-lucide-folder"
        class="size-3.5 text-muted"
      />
      {{ workspaces.active?.name ?? 'No workspace' }}
      <UIcon
        name="i-lucide-chevron-down"
        class="size-3 text-dimmed"
      />
    </button>
  </UDropdownMenu>
</template>
