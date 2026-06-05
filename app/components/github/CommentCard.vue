<script setup lang="ts">
const props = defineProps<{
  author: string
  avatarUrl: string | null
  association?: string | null
  createdAt: string
  body: string
  /** review verdict shown in the header, e.g. approved */
  verdict?: string | null
  /** PR author login, for the Author chip */
  prAuthor?: string
}>()

const verdictBadge = computed(() => {
  switch (props.verdict) {
    case 'approved':
      return { label: 'approved these changes', class: 'text-green-500' }
    case 'changes_requested':
      return { label: 'requested changes', class: 'text-red-400' }
    case 'dismissed':
      return { label: 'review dismissed', class: 'text-dimmed' }
    case 'commented':
      return { label: 'reviewed', class: 'text-dimmed' }
    default:
      return null
  }
})

const chips = computed(() => {
  const out: string[] = []
  if (props.association === 'OWNER') out.push('Owner')
  else if (props.association === 'MEMBER') out.push('Member')
  else if (props.association === 'COLLABORATOR') out.push('Collaborator')
  if (props.prAuthor && props.author === props.prAuthor) out.push('Author')
  return out
})
</script>

<template>
  <div class="flex gap-2.5">
    <UAvatar
      v-if="avatarUrl"
      :src="avatarUrl"
      :alt="author"
      size="2xs"
      class="mt-1 shrink-0"
    />
    <UIcon
      v-else
      name="i-lucide-user"
      class="size-5 mt-1 shrink-0 text-dimmed"
    />

    <div class="min-w-0 flex-1 rounded-lg border border-default overflow-hidden">
      <!-- header bar, GitHub-style -->
      <div class="flex items-center gap-1.5 px-3 py-1.5 bg-muted border-b border-default text-[11px]">
        <span class="font-semibold text-toned">{{ author }}</span>
        <span
          v-if="verdictBadge"
          :class="verdictBadge.class"
        >{{ verdictBadge.label }}</span>
        <span
          v-else
          class="text-dimmed"
        >commented</span>
        <span class="text-dimmed">{{ useRelativeDate(createdAt) }}</span>
        <span class="flex-1" />
        <UBadge
          v-for="chip in chips"
          :key="chip"
          color="neutral"
          variant="outline"
          size="sm"
        >
          {{ chip }}
        </UBadge>
      </div>

      <div
        v-if="body"
        class="px-3 py-2.5"
      >
        <MarkdownBody :source="body" />
      </div>
    </div>
  </div>
</template>
