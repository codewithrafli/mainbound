<script setup lang="ts">
const ui = useUiStore()
const git = useGitStore()
const github = useGithubStore()
const cockpit = useCockpitStore()

// poll PR/CI/review state while a PR exists (light, GH only)
let timer: ReturnType<typeof setInterval> | undefined
onMounted(() => {
  timer = setInterval(() => {
    if (document.visibilityState === 'visible' && cockpit.activeRepo && cockpit.pr) {
      cockpit.refreshGh(cockpit.activeRepo)
    }
  }, 30_000)
})
onBeforeUnmount(() => clearInterval(timer))

function openChanges() {
  if (!cockpit.activeRepo) return
  git.selectRepo(cockpit.activeRepo)
  ui.view = 'changes'
}

function openPr() {
  if (!cockpit.pr || !cockpit.activeRepo) return
  git.selectRepo(cockpit.activeRepo)
  github.openPrDetail(cockpit.activeRepo, cockpit.pr.number)
  ui.view = 'changes'
}

async function doPush() {
  if (!cockpit.activeRepo) return
  if (await github.push(cockpit.activeRepo)) cockpit.refresh()
}

async function doPull() {
  if (!cockpit.activeRepo) return
  if (await github.pull(cockpit.activeRepo)) cockpit.refresh()
}

const ciSummary = computed(() => {
  const c = cockpit.checks
  if (!c || !c.total) return null
  return {
    label: `${c.passed}/${c.total}`,
    color: c.failed ? 'text-red-400' : c.pending ? 'text-amber-400' : 'text-green-500',
    dot: c.failed ? 'bg-red-500' : c.pending ? 'bg-amber-400' : 'bg-green-500'
  }
})
</script>

<template>
  <div class="flex items-center h-7 shrink-0 gap-3 px-3 border-b border-default bg-muted text-[11px] text-muted select-none overflow-x-auto whitespace-nowrap">
    <template v-if="cockpit.status">
      <!-- branch -->
      <span class="flex items-center gap-1 text-toned">
        <UIcon
          name="i-lucide-git-branch"
          class="size-3"
        />
        {{ cockpit.status.branch ?? 'detached' }}
      </span>

      <!-- local changes -->
      <button
        class="flex items-center gap-1 hover:text-highlighted transition-colors"
        :class="cockpit.changeCount ? 'text-amber-400' : ''"
        title="Open File Changes (⌘2)"
        @click="openChanges"
      >
        <UIcon
          name="i-lucide-file-diff"
          class="size-3"
        />
        {{ cockpit.changeCount ? `${cockpit.changeCount} changed` : 'clean' }}
      </button>

      <!-- ahead / behind + sync -->
      <span class="flex items-center gap-1">
        <span
          v-if="cockpit.status.ahead"
          class="text-green-500"
          title="Unpushed commits"
        >↑{{ cockpit.status.ahead }}</span>
        <span
          v-if="cockpit.status.behind"
          class="text-amber-400"
        >↓{{ cockpit.status.behind }}</span>
        <span
          v-if="!cockpit.status.ahead && !cockpit.status.behind"
          class="text-dimmed"
        >synced</span>
        <UButton
          v-if="cockpit.status.behind"
          icon="i-lucide-arrow-down-to-line"
          color="neutral"
          variant="ghost"
          size="xs"
          class="-my-1"
          aria-label="Pull"
          :loading="github.syncing === 'pull'"
          @click="doPull"
        />
        <UButton
          v-if="cockpit.status.ahead"
          icon="i-lucide-arrow-up-from-line"
          color="neutral"
          variant="ghost"
          size="xs"
          class="-my-1"
          aria-label="Push"
          :loading="github.syncing === 'push'"
          @click="doPush"
        />
      </span>

      <!-- conflicts -->
      <span
        v-if="cockpit.status.conflicts.length"
        class="flex items-center gap-1 text-orange-500"
      >
        <UIcon
          name="i-lucide-triangle-alert"
          class="size-3"
        />
        {{ cockpit.status.conflicts.length }} conflict{{ cockpit.status.conflicts.length === 1 ? '' : 's' }}
      </span>

      <span class="flex-1" />

      <!-- PR + reviews + CI -->
      <template v-if="cockpit.pr">
        <button
          class="flex items-center gap-1.5 hover:text-highlighted transition-colors"
          title="Open pull request"
          @click="openPr"
        >
          <UIcon
            name="i-lucide-git-pull-request"
            class="size-3"
            :class="cockpit.pr.draft ? 'text-dimmed' : 'text-green-500'"
          />
          <span class="text-toned">#{{ cockpit.pr.number }}</span>
          <span class="max-w-48 truncate">{{ cockpit.pr.title }}</span>
        </button>

        <span
          v-if="cockpit.reviews"
          class="flex items-center gap-1.5"
          title="Reviews: approved / changes requested / commented"
        >
          <span
            v-if="cockpit.reviews.approved"
            class="text-green-500"
          >✓{{ cockpit.reviews.approved }}</span>
          <span
            v-if="cockpit.reviews.changes_requested"
            class="text-red-400"
          >±{{ cockpit.reviews.changes_requested }}</span>
          <span
            v-if="cockpit.reviews.commented"
            class="text-dimmed"
          >💬{{ cockpit.reviews.commented }}</span>
          <span
            v-if="!cockpit.reviews.approved && !cockpit.reviews.changes_requested && !cockpit.reviews.commented"
            class="text-dimmed"
          >no reviews</span>
        </span>

        <span
          v-if="ciSummary"
          class="flex items-center gap-1"
          :class="ciSummary.color"
          title="CI checks passed/total"
        >
          <span
            class="size-1.5 rounded-full"
            :class="ciSummary.dot"
          />
          CI {{ ciSummary.label }}
        </span>
      </template>

      <span
        v-else-if="github.user"
        class="text-dimmed"
      >no open PR</span>

      <UButton
        icon="i-lucide-refresh-cw"
        color="neutral"
        variant="ghost"
        size="xs"
        class="-my-1"
        aria-label="Refresh cockpit"
        :loading="cockpit.refreshing"
        @click="cockpit.refresh()"
      />
    </template>

    <template v-else>
      <span class="text-dimmed flex items-center gap-1.5">
        <UIcon
          name="i-lucide-git-branch"
          class="size-3"
        />
        {{ cockpit.activeCwd ? 'not a git repository' : 'no active session' }}
      </span>
    </template>
  </div>
</template>
