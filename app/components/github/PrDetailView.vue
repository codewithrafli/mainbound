<script setup lang="ts">
import type { CheckRun, ReviewThread, TimelineItem } from '~/stores/github'

const github = useGithubStore()
const git = useGitStore()

const tab = ref<'conversation' | 'checks'>('conversation')
const newComment = ref('')
const commenting = ref(false)
const mergeOpen = ref(false)
const mergeMethod = ref<'merge' | 'squash' | 'rebase'>('merge')
const merging = ref(false)

const pr = computed(() => github.prDetail)

type Entry
  = | { type: 'timeline', at: string, item: TimelineItem }
    | { type: 'thread', at: string, thread: ReviewThread }

/** Timeline items and review threads interleaved by time, GitHub-style. */
const conversation = computed<Entry[]>(() => {
  if (!pr.value) return []
  const entries: Entry[] = [
    ...pr.value.timeline.map(item => ({
      type: 'timeline' as const,
      at: item.created_at,
      item
    })),
    ...pr.value.threads.map(thread => ({
      type: 'thread' as const,
      at: thread.comments[0]?.created_at ?? '',
      thread
    }))
  ]
  return entries.sort((a, b) => a.at.localeCompare(b.at))
})

const conversationCount = computed(() =>
  conversation.value.filter(e =>
    e.type === 'thread' || e.item.kind === 'comment' || e.item.kind === 'review'
  ).length
)

function eventLabel(item: TimelineItem): string {
  switch (item.event) {
    case 'merged': return 'merged this pull request'
    case 'closed': return 'closed this'
    case 'reopened': return 'reopened this'
    case 'head_ref_force_pushed': return 'force-pushed the branch'
    case 'review_requested':
      return item.body ? `requested a review from ${item.body}` : 'requested a review'
    default: return item.event ?? ''
  }
}

function eventIcon(item: TimelineItem): { icon: string, cls: string } {
  switch (item.event) {
    case 'merged': return { icon: 'i-lucide-git-merge', cls: 'text-purple-400' }
    case 'closed': return { icon: 'i-lucide-circle-x', cls: 'text-red-400' }
    case 'reopened': return { icon: 'i-lucide-circle-dot', cls: 'text-green-500' }
    case 'head_ref_force_pushed': return { icon: 'i-lucide-zap', cls: 'text-amber-400' }
    case 'review_requested': return { icon: 'i-lucide-eye', cls: 'text-dimmed' }
    default: return { icon: 'i-lucide-circle', cls: 'text-dimmed' }
  }
}

async function openInBrowser() {
  if (!pr.value) return
  const { openUrl } = await import('@tauri-apps/plugin-opener')
  await openUrl(pr.value.html_url)
}

async function submitComment() {
  const text = newComment.value.trim()
  if (!text || commenting.value) return
  commenting.value = true
  if (await github.commentOnPr(text)) newComment.value = ''
  commenting.value = false
}

async function doMerge() {
  merging.value = true
  const ok = await github.mergePr(mergeMethod.value)
  merging.value = false
  mergeOpen.value = false
  if (ok && github.prDetailRepo) git.refresh(github.prDetailRepo)
}

function checkDot(check: CheckRun): string {
  if (check.status !== 'completed') return 'bg-amber-400'
  switch (check.conclusion) {
    case 'success':
    case 'neutral':
    case 'skipped':
      return 'bg-green-500'
    case 'failure':
    case 'timed_out':
    case 'cancelled':
    case 'action_required':
      return 'bg-red-500'
    default:
      return 'bg-neutral-600'
  }
}

async function openCheck(check: CheckRun) {
  if (!check.url) return
  const { openUrl } = await import('@tauri-apps/plugin-opener')
  await openUrl(check.url)
}

const stateBadge = computed(() => {
  if (!pr.value) return { label: '', color: 'neutral' as const }
  if (pr.value.merged) return { label: 'Merged', color: 'info' as const }
  if (pr.value.state === 'closed') return { label: 'Closed', color: 'error' as const }
  if (pr.value.draft) return { label: 'Draft', color: 'neutral' as const }
  return { label: 'Open', color: 'success' as const }
})
</script>

<template>
  <section class="flex-1 min-w-0 flex flex-col bg-default">
    <div
      v-if="github.loadingDetail && !pr"
      class="flex flex-1 items-center justify-center"
    >
      <UIcon
        name="i-lucide-loader-circle"
        class="size-5 animate-spin text-dimmed"
      />
    </div>

    <template v-else-if="pr">
      <!-- header -->
      <div class="shrink-0 border-b border-default px-4 py-3 space-y-2">
        <div class="flex items-start gap-2">
          <UBadge
            :color="stateBadge.color"
            variant="soft"
            size="sm"
            class="mt-0.5"
          >
            {{ stateBadge.label }}
          </UBadge>
          <h2 class="flex-1 min-w-0 text-sm font-medium text-highlighted leading-snug">
            {{ pr.title }}
            <span class="text-dimmed font-normal">#{{ pr.number }}</span>
          </h2>
          <UButton
            icon="i-lucide-refresh-cw"
            color="neutral"
            variant="ghost"
            size="xs"
            aria-label="Refresh"
            :loading="github.loadingDetail"
            @click="github.refreshPrDetail()"
          />
          <UButton
            icon="i-lucide-external-link"
            color="neutral"
            variant="ghost"
            size="xs"
            aria-label="Open in browser"
            @click="openInBrowser"
          />
          <UButton
            icon="i-lucide-x"
            color="neutral"
            variant="ghost"
            size="xs"
            aria-label="Close"
            @click="github.closePrDetail()"
          />
        </div>

        <div class="flex items-center gap-3 text-[11px] text-muted flex-wrap">
          <span class="flex items-center gap-1">
            <UAvatar
              v-if="pr.author_avatar"
              :src="pr.author_avatar"
              :alt="pr.author"
              size="3xs"
            />
            {{ pr.author }}
          </span>
          <span class="font-mono">{{ pr.head_ref }} → {{ pr.base_ref }}</span>
          <span class="text-green-500">+{{ pr.additions }}</span>
          <span class="text-red-500">-{{ pr.deletions }}</span>
          <span>{{ pr.commits }} commit{{ pr.commits === 1 ? '' : 's' }}</span>
          <span>{{ pr.changed_files }} files</span>

          <span class="flex-1" />

          <!-- merge: explicit confirmation, never one-click -->
          <UPopover
            v-if="!pr.merged && pr.state === 'open' && !pr.draft"
            v-model:open="mergeOpen"
          >
            <UButton
              label="Merge"
              icon="i-lucide-git-merge"
              color="success"
              variant="soft"
              size="xs"
              :disabled="pr.mergeable === false"
              :title="pr.mergeable === false ? 'Has conflicts — resolve first' : 'Merge pull request'"
            />
            <template #content>
              <div class="p-3 w-64 space-y-2">
                <p class="text-xs text-toned">
                  Merge <span class="font-mono text-highlighted">#{{ pr.number }}</span> into
                  <span class="font-mono text-highlighted">{{ pr.base_ref }}</span>?
                </p>
                <USelect
                  v-model="mergeMethod"
                  :items="[
                    { label: 'Create a merge commit', value: 'merge' },
                    { label: 'Squash and merge', value: 'squash' },
                    { label: 'Rebase and merge', value: 'rebase' }
                  ]"
                  size="xs"
                  class="w-full"
                />
                <UButton
                  label="Confirm merge"
                  color="success"
                  size="xs"
                  block
                  :loading="merging"
                  @click="doMerge"
                />
              </div>
            </template>
          </UPopover>
        </div>

        <!-- tabs -->
        <div class="flex gap-1">
          <UButton
            label="Conversation"
            :badge="conversationCount || undefined"
            color="neutral"
            :variant="tab === 'conversation' ? 'soft' : 'ghost'"
            size="xs"
            @click="tab = 'conversation'"
          />
          <UButton
            :label="`Checks (${pr.checks.length})`"
            color="neutral"
            :variant="tab === 'checks' ? 'soft' : 'ghost'"
            size="xs"
            @click="tab = 'checks'"
          />
        </div>
      </div>

      <!-- conversation -->
      <div
        v-show="tab === 'conversation'"
        class="flex-1 min-h-0 overflow-y-auto px-4 py-3"
      >
        <div class="max-w-3xl space-y-3">
          <!-- PR description as the first card -->
          <GithubCommentCard
            :author="pr.author"
            :avatar-url="pr.author_avatar"
            :created-at="''"
            :body="pr.body || '*No description provided.*'"
            :pr-author="pr.author"
          />

          <template
            v-for="(entry, i) in conversation"
            :key="i"
          >
            <!-- comment / review cards -->
            <GithubCommentCard
              v-if="entry.type === 'timeline' && (entry.item.kind === 'comment' || entry.item.kind === 'review')"
              :author="entry.item.author"
              :avatar-url="entry.item.avatar_url"
              :association="entry.item.association"
              :created-at="entry.item.created_at"
              :body="entry.item.body"
              :verdict="entry.item.kind === 'review' ? entry.item.review_state : null"
              :pr-author="pr.author"
            />

            <!-- commit row -->
            <div
              v-else-if="entry.type === 'timeline' && entry.item.kind === 'commit'"
              class="flex items-center gap-2 pl-1 text-[11px] text-muted"
            >
              <UIcon
                name="i-lucide-git-commit-horizontal"
                class="size-3.5 shrink-0 text-dimmed"
              />
              <span class="truncate font-mono text-toned">{{ entry.item.body }}</span>
              <span class="flex-1" />
              <span class="font-mono text-dimmed">{{ entry.item.sha }}</span>
            </div>

            <!-- event row -->
            <div
              v-else-if="entry.type === 'timeline' && entry.item.kind === 'event'"
              class="flex items-center gap-2 pl-1 text-[11px] text-muted"
            >
              <UIcon
                :name="eventIcon(entry.item).icon"
                class="size-3.5 shrink-0"
                :class="eventIcon(entry.item).cls"
              />
              <span><span class="font-medium text-toned">{{ entry.item.author }}</span> {{ eventLabel(entry.item) }}</span>
              <span class="text-dimmed">{{ useRelativeDate(entry.item.created_at) }}</span>
            </div>

            <!-- inline review thread -->
            <GithubThreadCard
              v-else-if="entry.type === 'thread'"
              :thread="entry.thread"
              :pr-author="pr.author"
            />
          </template>
        </div>
      </div>

      <!-- checks -->
      <div
        v-show="tab === 'checks'"
        class="flex-1 min-h-0 overflow-y-auto px-4 py-3 space-y-1"
      >
        <button
          v-for="check in pr.checks"
          :key="check.name"
          class="flex items-center gap-2 w-full px-2 py-1.5 rounded-md text-left text-[12px] text-toned hover:bg-elevated/50 transition-colors"
          @click="openCheck(check)"
        >
          <span
            class="size-1.5 shrink-0 rounded-full"
            :class="checkDot(check)"
          />
          <span class="flex-1 truncate">{{ check.name }}</span>
          <span class="text-[10px] text-dimmed">{{ check.conclusion ?? check.status }}</span>
          <UIcon
            v-if="check.url"
            name="i-lucide-external-link"
            class="size-3 text-dimmed"
          />
        </button>
        <p
          v-if="!pr.checks.length"
          class="text-xs text-dimmed italic"
        >
          No CI checks on this commit.
        </p>
      </div>

      <!-- composer -->
      <div
        v-show="tab === 'conversation'"
        class="shrink-0 border-t border-default p-3 space-y-2"
      >
        <UTextarea
          v-model="newComment"
          placeholder="Write a comment… (⌘↵ to send)"
          :rows="2"
          size="sm"
          class="w-full"
          @keydown.meta.enter="submitComment"
        />
        <div class="flex justify-end">
          <UButton
            label="Comment"
            color="primary"
            size="xs"
            :loading="commenting"
            :disabled="!newComment.trim()"
            @click="submitComment"
          />
        </div>
      </div>
    </template>
  </section>
</template>
