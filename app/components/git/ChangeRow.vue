<script setup lang="ts">
import type { FileChange } from '~/stores/git'

defineProps<{
  file: FileChange
  active?: boolean
}>()

defineEmits<{
  select: []
  action: []
}>()
</script>

<template>
  <div
    role="button"
    tabindex="0"
    class="group flex items-center gap-1.5 w-full px-2 py-1 rounded text-[12px] cursor-pointer transition-colors"
    :class="active
      ? 'bg-elevated text-highlighted'
      : 'text-toned hover:bg-elevated/50'"
    @click="$emit('select')"
    @keydown.enter="$emit('select')"
  >
    <GitStatusBadge :status="file.status" />
    <span
      class="flex-1 truncate"
      :title="file.path"
    >
      {{ file.path.split('/').pop() }}
      <span class="text-dimmed text-[10px] ml-0.5">{{ file.path.includes('/') ? file.path.slice(0, file.path.lastIndexOf('/')) : '' }}</span>
    </span>
    <span
      v-if="file.added !== null"
      class="text-[10px] font-mono text-green-500"
    >+{{ file.added }}</span>
    <span
      v-if="file.removed !== null && file.removed > 0"
      class="text-[10px] font-mono text-red-500"
    >-{{ file.removed }}</span>
    <UButton
      :icon="file.staged ? 'i-lucide-minus' : 'i-lucide-plus'"
      color="neutral"
      variant="ghost"
      size="xs"
      class="opacity-0 group-hover:opacity-100 -my-1"
      :aria-label="file.staged ? 'Unstage' : 'Stage'"
      @click.stop="$emit('action')"
    />
  </div>
</template>
