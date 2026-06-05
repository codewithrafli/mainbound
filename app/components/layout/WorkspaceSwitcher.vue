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
    <UButton
      :label="workspaces.active?.name ?? 'No workspace'"
      trailing-icon="i-lucide-chevron-down"
      color="neutral"
      variant="outline"
      size="xs"
      class="rounded-full px-3"
    />
  </UDropdownMenu>
</template>
