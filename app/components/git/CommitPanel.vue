<script setup lang="ts">
const git = useGitStore()

const summary = ref('')
const description = ref('')

const stagedOpen = ref(true)
const commitOpen = ref(true)
const historyOpen = ref(true)

const canCommit = computed(() =>
  !!summary.value.trim() && !!git.status?.staged.length && !git.committing
)

async function doCommit() {
  if (!canCommit.value || !git.selectedRepo) return
  const ok = await git.commit(git.selectedRepo, summary.value.trim(), description.value.trim())
  if (ok) {
    summary.value = ''
    description.value = ''
  }
}

function relativeDate(iso: string): string {
  const then = new Date(iso).getTime()
  const days = Math.floor((Date.now() - then) / 86_400_000)
  if (days === 0) return 'today'
  if (days === 1) return 'yesterday'
  if (days < 30) return `${days}d ago`
  return new Date(iso).toLocaleDateString(undefined, { day: 'numeric', month: 'short' })
}
</script>

<template>
  <aside class="flex flex-col w-72 shrink-0 border-l border-default bg-muted overflow-y-auto">
    <template v-if="git.status">
      <!-- branch header -->
      <div class="flex items-center gap-2 px-3 py-2.5 border-b border-default">
        <UIcon name="i-lucide-git-branch" class="size-3.5 text-muted" />
        <span class="text-sm font-medium text-highlighted truncate">{{ git.status.branch ?? 'detached' }}</span>
        <span v-if="git.status.oid" class="text-[10px] font-mono text-dimmed">{{ git.status.oid }}</span>
        <UButton
          icon="i-lucide-refresh-cw"
          color="neutral"
          variant="ghost"
          size="xs"
          class="ml-auto"
          aria-label="Refresh"
          @click="git.refresh(git.selectedRepo!)"
        />
      </div>

      <!-- STAGED -->
      <button
        class="flex items-center gap-1.5 px-3 pt-3 pb-1.5 text-[10px] font-medium tracking-wider text-dimmed uppercase hover:text-muted"
        @click="stagedOpen = !stagedOpen"
      >
        <UIcon :name="stagedOpen ? 'i-lucide-chevron-down' : 'i-lucide-chevron-right'" class="size-3" />
        Staged
        <UBadge color="neutral" variant="soft" size="sm">{{ git.status.staged.length }}</UBadge>
        <UButton
          label="Stage all"
          color="neutral"
          variant="link"
          size="xs"
          class="ml-auto"
          :disabled="!git.status.unstaged.length"
          @click.stop="git.stageAll(git.selectedRepo!)"
        />
      </button>

      <div v-show="stagedOpen" class="px-2 space-y-0.5">
        <GitChangeRow
          v-for="file in git.status.staged"
          :key="`s-${file.path}`"
          :file="file"
          :active="git.selected?.file.path === file.path && git.selected?.file.staged"
          @select="git.selectFile(git.selectedRepo!, file)"
          @action="git.unstage(git.selectedRepo!, [file.path])"
        />
        <p v-if="!git.status.staged.length" class="px-2 py-2 text-[11px] text-dimmed italic">
          Nothing staged yet — use the Changes list on the left or "Stage all" above.
        </p>
      </div>

      <!-- COMMIT -->
      <button
        class="flex items-center gap-1.5 px-3 pt-4 pb-1.5 text-[10px] font-medium tracking-wider text-dimmed uppercase hover:text-muted"
        @click="commitOpen = !commitOpen"
      >
        <UIcon :name="commitOpen ? 'i-lucide-chevron-down' : 'i-lucide-chevron-right'" class="size-3" />
        <UIcon name="i-lucide-git-commit-horizontal" class="size-3" />
        Commit
      </button>

      <div v-show="commitOpen" class="px-3 space-y-2">
        <UInput
          v-model="summary"
          placeholder="Summary (required)"
          size="sm"
          class="w-full"
          @keydown.meta.enter="doCommit"
        />
        <UTextarea
          v-model="description"
          placeholder="Description (optional)"
          :rows="3"
          size="sm"
          class="w-full"
          @keydown.meta.enter="doCommit"
        />
        <UButton
          label="Commit"
          color="primary"
          size="sm"
          block
          :loading="git.committing"
          :disabled="!canCommit"
          @click="doCommit"
        />
        <p class="text-[10px] text-dimmed text-center">⌘↵ to commit</p>
        <UAlert
          v-if="git.error"
          color="error"
          variant="soft"
          :description="git.error"
          class="text-xs"
        />
      </div>

      <!-- HISTORY -->
      <button
        class="flex items-center gap-1.5 px-3 pt-4 pb-1.5 text-[10px] font-medium tracking-wider text-dimmed uppercase hover:text-muted"
        @click="historyOpen = !historyOpen"
      >
        <UIcon :name="historyOpen ? 'i-lucide-chevron-down' : 'i-lucide-chevron-right'" class="size-3" />
        History
        <UBadge color="neutral" variant="soft" size="sm">{{ git.log.length }}</UBadge>
      </button>

      <div v-show="historyOpen" class="px-3 pb-3 space-y-2.5">
        <div v-for="commit in git.log" :key="commit.hash" class="flex gap-2">
          <span class="mt-1 size-1.5 shrink-0 rounded-full bg-blue-500" />
          <div class="min-w-0">
            <p class="text-[12px] text-toned truncate leading-tight">{{ commit.subject }}</p>
            <p class="text-[10px] text-dimmed font-mono leading-tight pt-0.5">
              {{ commit.short_hash }} · {{ commit.author }} · {{ relativeDate(commit.date) }}
            </p>
          </div>
        </div>
        <p v-if="!git.log.length" class="text-[11px] text-dimmed italic">
          No commits yet.
        </p>
      </div>
    </template>

    <div v-else class="flex flex-1 items-center justify-center p-6">
      <p class="text-xs text-dimmed text-center">
        Select a repository to stage and commit changes.
      </p>
    </div>
  </aside>
</template>
