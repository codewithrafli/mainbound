<script setup lang="ts">
import type { FileChange } from '~/stores/git'

const props = defineProps<{
  file: FileChange
  active?: boolean
}>()

const emit = defineEmits<{
  select: []
  action: []
  discard: []
}>()

const confirmOpen = ref(false)

const fileName = computed(() => props.file.path.split('/').pop())

function doDiscard() {
  confirmOpen.value = false
  emit('discard')
}
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
      {{ fileName }}
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

    <!-- discard (unstaged only): destructive, asks for explicit confirmation -->
    <UPopover
      v-if="!file.staged"
      v-model:open="confirmOpen"
    >
      <UButton
        icon="i-lucide-undo-2"
        color="neutral"
        variant="ghost"
        size="xs"
        class="opacity-0 group-hover:opacity-100 -my-1 hover:text-red-400"
        aria-label="Discard changes"
        @click.stop="confirmOpen = true"
      />
      <template #content>
        <div
          class="p-3 w-60 space-y-2"
          @click.stop
        >
          <p class="text-xs text-toned">
            Discard changes to <span class="font-mono text-highlighted">{{ fileName }}</span>?
            <template v-if="file.status === 'U'">
              The file will be <span class="text-red-400">deleted</span>.
            </template>
            This cannot be undone.
          </p>
          <UButton
            label="Discard"
            color="error"
            size="xs"
            block
            @click="doDiscard"
          />
        </div>
      </template>
    </UPopover>

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
