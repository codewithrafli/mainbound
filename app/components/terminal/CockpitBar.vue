<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import type { DropdownMenuItem } from '@nuxt/ui'

const ui = useUiStore()
const git = useGitStore()
const github = useGithubStore()
const cockpit = useCockpitStore()
const toast = useToast()

// --- branch switcher ------------------------------------------------------

interface BranchInfo {
  name: string
  current: boolean
}

const branches = ref<BranchInfo[]>([])
const newBranchOpen = ref(false)
const newBranchName = ref('')
const switching = ref(false)

async function loadBranches(open: boolean) {
  if (!open || !cockpit.activeRepo) return
  branches.value = await invoke<BranchInfo[]>('git_branches', { repo: cockpit.activeRepo })
    .catch(() => [])
}

async function checkout(branch: string, create = false) {
  if (!cockpit.activeRepo || switching.value) return
  switching.value = true
  try {
    await invoke('git_checkout', { repo: cockpit.activeRepo, branch, create })
    newBranchOpen.value = false
    newBranchName.value = ''
    await cockpit.refresh()
  } catch (e) {
    // surface the real git error, per UX principles
    toast.add({ title: 'Checkout failed', description: String(e), color: 'error', icon: 'i-lucide-git-branch' })
  } finally {
    switching.value = false
  }
}

const branchItems = computed<DropdownMenuItem[][]>(() => [
  branches.value.map(b => ({
    label: b.name,
    type: 'checkbox' as const,
    checked: b.current,
    onSelect: () => {
      if (!b.current) checkout(b.name)
    }
  })),
  [{
    label: 'New branch…',
    icon: 'i-lucide-git-branch-plus',
    onSelect: () => {
      newBranchOpen.value = true
    }
  }]
])

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
  <div class="flex items-center h-9 shrink-0 gap-1.5 px-3 border-b border-(--ui-border-muted) text-[10.5px] font-mono text-muted select-none overflow-x-auto whitespace-nowrap">
    <template v-if="cockpit.status">
      <!-- branch switcher -->
      <UDropdownMenu
        :items="branchItems"
        :content="{ align: 'start' }"
        :ui="{ content: 'w-60 max-h-72 overflow-y-auto' }"
        @update:open="loadBranches"
      >
        <button class="flex items-center gap-1.5 rounded-full border border-(--ui-border) bg-muted/60 px-2.5 py-1 text-toned hover:bg-elevated transition-colors">
          <UIcon
            :name="switching ? 'i-lucide-loader-circle' : 'i-lucide-git-branch'"
            class="size-3"
            :class="switching ? 'animate-spin' : ''"
          />
          {{ cockpit.status.branch ?? 'detached' }}
          <UIcon
            name="i-lucide-chevron-down"
            class="size-2.5 text-dimmed"
          />
        </button>
      </UDropdownMenu>

      <!-- new branch -->
      <UModal
        v-model:open="newBranchOpen"
        title="New Branch"
        :ui="{ content: 'max-w-sm' }"
      >
        <template #body>
          <div class="space-y-2">
            <p class="text-xs text-muted">
              Create from <span class="font-mono text-toned">{{ cockpit.status?.branch }}</span> and switch to it.
            </p>
            <UInput
              v-model="newBranchName"
              placeholder="feat/my-branch"
              size="sm"
              class="w-full font-mono"
              autofocus
              @keydown.enter="newBranchName.trim() && checkout(newBranchName.trim(), true)"
            />
            <UButton
              label="Create & Switch"
              color="neutral"
              variant="solid"
              size="sm"
              block
              :loading="switching"
              :disabled="!newBranchName.trim()"
              @click="checkout(newBranchName.trim(), true)"
            />
          </div>
        </template>
      </UModal>

      <!-- changes chip -->
      <button
        class="flex items-center gap-1.5 rounded-full border border-(--ui-border) px-2.5 py-1 transition-colors hover:bg-elevated"
        :class="cockpit.changeCount ? 'text-amber-400 border-amber-400/20' : 'text-dimmed'"
        title="Open File Changes (⌘2)"
        @click="openChanges"
      >
        <UIcon
          name="i-lucide-file-diff"
          class="size-3"
        />
        {{ cockpit.changeCount ? `${cockpit.changeCount} changed` : 'clean' }}
      </button>

      <!-- sync chip -->
      <span class="flex items-center gap-1 rounded-full border border-(--ui-border) px-2.5 py-1">
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
          class="-my-1.5 -mr-1"
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
          class="-my-1.5 -mr-1"
          aria-label="Push"
          :loading="github.syncing === 'push'"
          @click="doPush"
        />
      </span>

      <!-- conflicts -->
      <span
        v-if="cockpit.status.conflicts.length"
        class="flex items-center gap-1.5 rounded-full border border-orange-500/30 px-2.5 py-1 text-orange-400"
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
          class="flex items-center gap-1.5 rounded-full border border-(--ui-border) px-2.5 py-1 transition-colors hover:bg-elevated"
          title="Open pull request"
          @click="openPr"
        >
          <UIcon
            name="i-lucide-git-pull-request"
            class="size-3"
            :class="cockpit.pr.draft ? 'text-dimmed' : 'text-green-500'"
          />
          <span class="text-toned">#{{ cockpit.pr.number }}</span>
          <span class="max-w-44 truncate">{{ cockpit.pr.title }}</span>
        </button>

        <span
          v-if="cockpit.reviews"
          class="flex items-center gap-1.5 rounded-full border border-(--ui-border) px-2.5 py-1"
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
          class="flex items-center gap-1.5 rounded-full border border-(--ui-border) px-2.5 py-1"
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
        class="text-dimmed px-1"
      >no open PR</span>

      <UButton
        icon="i-lucide-refresh-cw"
        color="neutral"
        variant="ghost"
        size="xs"
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
