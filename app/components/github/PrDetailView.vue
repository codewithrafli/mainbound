<script setup lang="ts">
import type { CheckRun, PrComment } from '~/stores/github'

const github = useGithubStore()
const git = useGitStore()

const tab = ref<'conversation' | 'checks'>('conversation')
const newComment = ref('')
const commenting = ref(false)
const mergeOpen = ref(false)
const mergeMethod = ref<'merge' | 'squash' | 'rebase'>('merge')
const merging = ref(false)

const pr = computed(() => github.prDetail)

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

function relativeDate(iso: string): string {
  if (!iso) return ''
  const mins = Math.floor((Date.now() - new Date(iso).getTime()) / 60_000)
  if (mins < 1) return 'just now'
  if (mins < 60) return `${mins}m ago`
  const hours = Math.floor(mins / 60)
  if (hours < 24) return `${hours}h ago`
  const days = Math.floor(hours / 24)
  if (days < 30) return `${days}d ago`
  return new Date(iso).toLocaleDateString(undefined, { day: 'numeric', month: 'short' })
}

function commentBadge(comment: PrComment): { label: string, class: string } | null {
  if (comment.kind === 'review:APPROVED') return { label: 'approved', class: 'text-green-500' }
  if (comment.kind === 'review:CHANGES_REQUESTED') return { label: 'requested changes', class: 'text-red-400' }
  if (comment.kind.startsWith('review:')) return { label: 'reviewed', class: 'text-dimmed' }
  return null
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
            :badge="pr.comments.length || undefined"
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
        class="flex-1 min-h-0 overflow-y-auto px-4 py-3 space-y-4"
      >
        <!-- PR body -->
        <div
          v-if="pr.body"
          class="rounded-lg border border-default bg-muted p-3"
        >
          <p class="text-[12px] text-toned whitespace-pre-wrap break-words">
            {{ pr.body }}
          </p>
        </div>

        <div
          v-for="(comment, i) in pr.comments"
          :key="i"
          class="flex gap-2.5"
        >
          <UAvatar
            v-if="comment.avatar_url"
            :src="comment.avatar_url"
            :alt="comment.author"
            size="2xs"
            class="mt-0.5 shrink-0"
          />
          <UIcon
            v-else
            name="i-lucide-user"
            class="size-5 mt-0.5 shrink-0 text-dimmed"
          />
          <div class="min-w-0 flex-1">
            <p class="text-[11px] leading-tight">
              <span class="font-medium text-toned">{{ comment.author }}</span>
              <span
                v-if="commentBadge(comment)"
                class="ml-1.5"
                :class="commentBadge(comment)!.class"
              >{{ commentBadge(comment)!.label }}</span>
              <span class="text-dimmed ml-1.5">{{ relativeDate(comment.created_at) }}</span>
            </p>
            <p
              v-if="comment.path"
              class="text-[10px] font-mono text-dimmed pt-0.5 truncate"
            >
              {{ comment.path }}<template v-if="comment.line">
                :{{ comment.line }}
              </template>
            </p>
            <p
              v-if="comment.body"
              class="text-[12px] text-toned whitespace-pre-wrap break-words pt-1"
            >
              {{ comment.body }}
            </p>
          </div>
        </div>

        <p
          v-if="!pr.comments.length && !pr.body"
          class="text-xs text-dimmed italic"
        >
          No conversation yet.
        </p>
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
