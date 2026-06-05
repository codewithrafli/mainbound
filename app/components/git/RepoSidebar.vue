<script setup lang="ts">
const workspaces = useWorkspacesStore()
const git = useGitStore()

const reposOpen = ref(true)
const changesOpen = ref(true)

const conflictCount = computed(() => git.status?.conflicts.length ?? 0)
</script>

<template>
  <aside class="flex flex-col w-64 shrink-0 border-r border-default bg-muted overflow-y-auto">
    <!-- REPOSITORIES -->
    <button
      class="flex items-center gap-1.5 px-3 pt-3 pb-1.5 text-[10px] font-medium tracking-wider text-dimmed uppercase hover:text-muted"
      @click="reposOpen = !reposOpen"
    >
      <UIcon
        :name="reposOpen ? 'i-lucide-chevron-down' : 'i-lucide-chevron-right'"
        class="size-3"
      />
      Repositories
      <UBadge
        color="neutral"
        variant="soft"
        size="sm"
      >
        {{ workspaces.repos.length }}
      </UBadge>
    </button>

    <nav
      v-show="reposOpen"
      class="px-2 space-y-0.5"
    >
      <button
        v-for="repo in workspaces.repos"
        :key="repo.path"
        class="flex items-center gap-2 w-full px-2 py-1.5 rounded-md text-left text-[12px] transition-colors"
        :class="repo.path === git.selectedRepo
          ? 'bg-elevated text-highlighted'
          : 'text-muted hover:bg-elevated/50 hover:text-toned'"
        @click="git.selectRepo(repo.path)"
      >
        <UIcon
          name="i-lucide-folder-git-2"
          class="size-3.5 shrink-0 text-blue-400"
        />
        <span class="flex-1 truncate font-medium">{{ repo.name }}</span>
        <span
          v-if="repo.branch"
          class="truncate max-w-24 text-[10px] text-dimmed"
        >{{ repo.branch }}</span>
        <UBadge
          v-if="git.changeCount(repo.path)"
          color="neutral"
          variant="soft"
          size="sm"
        >
          {{ git.changeCount(repo.path) }}
        </UBadge>
      </button>

      <p
        v-if="!workspaces.repos.length"
        class="px-2 py-3 text-xs text-dimmed italic"
      >
        No repositories found in this workspace.
      </p>
    </nav>

    <!-- CHANGES -->
    <button
      class="flex items-center gap-1.5 px-3 pt-4 pb-1.5 text-[10px] font-medium tracking-wider text-dimmed uppercase hover:text-muted"
      @click="changesOpen = !changesOpen"
    >
      <UIcon
        :name="changesOpen ? 'i-lucide-chevron-down' : 'i-lucide-chevron-right'"
        class="size-3"
      />
      Changes
      <UBadge
        color="neutral"
        variant="soft"
        size="sm"
      >
        {{ (git.status?.unstaged.length ?? 0) + (git.status?.staged.length ?? 0) }}
      </UBadge>
    </button>

    <div
      v-show="changesOpen"
      class="flex-1 px-2 space-y-0.5"
    >
      <template v-if="git.status">
        <GitChangeRow
          v-for="file in git.status.unstaged"
          :key="`u-${file.path}`"
          :file="file"
          :active="git.selected?.file.path === file.path && !git.selected?.file.staged"
          @select="git.selectFile(git.selectedRepo!, file)"
          @action="git.stage(git.selectedRepo!, [file.path])"
        />
        <p
          v-if="!git.status.unstaged.length"
          class="px-2 py-2 text-xs text-dimmed italic"
        >
          No unstaged changes.
        </p>
      </template>
      <p
        v-else
        class="px-2 py-2 text-xs text-dimmed italic"
      >
        Select a repository above.
      </p>
    </div>

    <!-- CONFLICTS -->
    <div class="mt-auto border-t border-default px-3 py-2.5">
      <div class="flex items-center gap-1.5 text-[10px] font-medium tracking-wider text-dimmed uppercase">
        <UIcon
          name="i-lucide-git-merge"
          class="size-3"
        />
        Conflicts
        <UBadge
          :color="conflictCount ? 'warning' : 'neutral'"
          variant="soft"
          size="sm"
        >
          {{ conflictCount }}
        </UBadge>
      </div>
      <p
        v-if="!conflictCount"
        class="pt-1.5 text-xs text-dimmed"
      >
        No merge in progress.
      </p>
    </div>
  </aside>
</template>
