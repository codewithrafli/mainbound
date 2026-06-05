<script setup lang="ts">
import type { ReviewThread } from '~/stores/github'

const props = defineProps<{
  thread: ReviewThread
  prAuthor: string
}>()

const github = useGithubStore()

const expanded = ref(!props.thread.resolved)
const reply = ref('')
const replying = ref(false)
const resolving = ref(false)

const rootHunk = computed(() => props.thread.comments[0]?.diff_hunk ?? null)

/** Last few lines of the diff hunk — enough context, GitHub-style. */
const hunkLines = computed(() => {
  if (!rootHunk.value) return []
  return rootHunk.value
    .split('\n')
    .filter(line => !line.startsWith('@@'))
    .slice(-4)
    .map(line => ({
      text: line,
      cls: line.startsWith('+')
        ? 'text-green-400 bg-green-500/10'
        : line.startsWith('-')
          ? 'text-red-400 bg-red-500/10'
          : 'text-muted'
    }))
})

function chips(association: string | null, author: string): string[] {
  const out: string[] = []
  if (association === 'OWNER') out.push('Owner')
  else if (association === 'MEMBER') out.push('Member')
  if (author === props.prAuthor) out.push('Author')
  return out
}

async function submitReply() {
  const text = reply.value.trim()
  const rootId = props.thread.comments[0]?.id
  if (!text || !rootId || replying.value) return
  replying.value = true
  if (await github.replyToThread(rootId, text)) reply.value = ''
  replying.value = false
}

async function toggleResolved() {
  resolving.value = true
  await github.resolveThread(props.thread.id, !props.thread.resolved)
  resolving.value = false
}
</script>

<template>
  <div class="rounded-lg border border-default overflow-hidden">
    <!-- file header -->
    <button
      class="flex items-center gap-2 w-full px-3 py-1.5 bg-muted text-left border-b border-default"
      @click="expanded = !expanded"
    >
      <UIcon
        :name="expanded ? 'i-lucide-chevron-down' : 'i-lucide-chevron-right'"
        class="size-3 shrink-0 text-dimmed"
      />
      <span class="font-mono text-[11px] text-toned truncate">
        {{ thread.path }}<template v-if="thread.line">:{{ thread.line }}</template>
      </span>
      <UBadge
        v-if="thread.outdated"
        color="warning"
        variant="soft"
        size="sm"
      >
        Outdated
      </UBadge>
      <span class="flex-1" />
      <UBadge
        v-if="thread.resolved"
        color="neutral"
        variant="soft"
        size="sm"
      >
        Resolved
      </UBadge>
    </button>

    <template v-if="expanded">
      <!-- diff context -->
      <pre
        v-if="hunkLines.length"
        class="text-[11px] font-mono leading-snug overflow-x-auto bg-[#0d0d0d] border-b border-default"
      ><div
        v-for="(hunkLine, i) in hunkLines"
        :key="i"
        class="px-3 whitespace-pre"
        :class="hunkLine.cls"
      >{{ hunkLine.text }}</div></pre>

      <!-- thread comments -->
      <div class="divide-y divide-(--ui-border)">
        <div
          v-for="comment in thread.comments"
          :key="comment.id"
          class="px-3 py-2.5"
        >
          <div class="flex items-center gap-1.5 text-[11px] pb-1">
            <UAvatar
              v-if="comment.avatar_url"
              :src="comment.avatar_url"
              :alt="comment.author"
              size="3xs"
            />
            <span class="font-semibold text-toned">{{ comment.author }}</span>
            <span class="text-dimmed">{{ useRelativeDate(comment.created_at) }}</span>
            <span class="flex-1" />
            <UBadge
              v-for="chip in chips(comment.association, comment.author)"
              :key="chip"
              color="neutral"
              variant="outline"
              size="sm"
            >
              {{ chip }}
            </UBadge>
          </div>
          <MarkdownBody :source="comment.body" />
        </div>
      </div>

      <!-- reply + resolve -->
      <div class="px-3 py-2.5 bg-muted/50 border-t border-default space-y-2">
        <UInput
          v-model="reply"
          placeholder="Reply…"
          size="sm"
          class="w-full"
          :loading="replying"
          @keydown.enter="submitReply"
        />
        <div class="flex items-center justify-between">
          <UButton
            :label="thread.resolved ? 'Unresolve conversation' : 'Resolve conversation'"
            color="neutral"
            variant="outline"
            size="xs"
            :loading="resolving"
            @click="toggleResolved"
          />
          <UButton
            v-if="reply.trim()"
            label="Reply"
            color="primary"
            size="xs"
            :loading="replying"
            @click="submitReply"
          />
        </div>
      </div>
    </template>
  </div>
</template>
