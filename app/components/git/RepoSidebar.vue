<script setup lang="ts">
const workspaces = useWorkspacesStore()
const git = useGitStore()

const reposOpen = ref(true)
const changesOpen = ref(true)

const conflictCount = computed(() => git.status?.conflicts.length ?? 0)
</script>


<template>
  <aside class="flex flex-col w-64 shrink-0 border-r border-default bg-muted/40 overflow-y-auto">
    <!-- Repositories -->
    <button
      class="flex items-center gap-1.5 px-3 pt-3 pb-1.5 section-label hover:text-toned"
      @click="reposOpen = !reposOpen"
    >
      <UIcon
        :name="reposOpen ? 'i-lucide-chevron-down' : 'i-lucide-chevron-right'"
        class="size-3"
      />
      Repositories
      <span class="font-mono text-dimmed">{{ workspaces.repos.length }}</span>
    </button>

    <nav
      v-show="reposOpen"
      class="px-2 space-y-0.5"
    >
      <button
        v-for="repo in workspaces.repos"
        :key="repo.path"
        class="flex items-center gap-2 w-full px-2 py-1.5 rounded-lg text-left text-[12.5px] transition-colors"
        :class="repo.path === git.selectedRepo
          ? 'bg-elevated text-highlighted ring-1 ring-(--ui-border-accented)'
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
          class="truncate max-w-20 text-[10px] font-mono text-dimmed"
        >{{ repo.branch }}</span>
        <span
          v-if="git.changeCount(repo.path)"
          class="rounded-full bg-accented px-1.5 text-[10px] font-mono text-toned"
        >
          {{ git.changeCount(repo.path) }}
        </span>
      </button>

      <p
        v-if="!workspaces.repos.length"
        class="px-2 py-3 text-xs text-dimmed italic"
      >
        No repositories found in this workspace.
      </p>
    </nav>

    <!-- Changes -->
    <button
      class="flex items-center gap-1.5 px-3 pt-4 pb-1.5 section-label hover:text-toned"
      @click="changesOpen = !changesOpen"
    >
      <UIcon
        :name="changesOpen ? 'i-lucide-chevron-down' : 'i-lucide-chevron-right'"
        class="size-3"
      />
      Changes
      <span class="font-mono text-dimmed">
        {{ (git.status?.unstaged.length ?? 0) + (git.status?.staged.length ?? 0) }}
      </span>
    </button>

    <!-- file search -->
    <div
      v-if="changesOpen && (git.status?.unstaged.length ?? 0) > 5"
      class="px-2 pb-1"
    >
      <UInput
        v-model="git.fileSearch"
        placeholder="Filter files…"
        size="xs"
        icon="i-lucide-search"
        class="w-full"
      />
    </div>

    <div
      v-show="changesOpen"
      class="flex-1 px-2 space-y-0.5"
    >
      <template v-if="git.status">
        <GitChangeRow
          v-for="file in git.filteredUnstaged"
          :key="`u-${file.path}`"
          :file="file"
          :active="git.selected?.file.path === file.path && !git.selected?.file.staged"
          @select="git.selectFile(git.selectedRepo!, file)"
          @action="git.stage(git.selectedRepo!, [file.path])"
          @discard="git.discard(git.selectedRepo!, file)"
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

    <!-- Conflicts card -->
    <div class="p-2">
      <div
        class="panel-card px-3 py-2.5"
        :class="conflictCount ? 'border-orange-500/30' : ''"
      >
        <div class="flex items-center gap-1.5 section-label">
          <UIcon
            name="i-lucide-git-merge"
            class="size-3"
          />
          Conflicts
          <span
            class="ml-auto font-mono"
            :class="conflictCount ? 'text-orange-400' : 'text-dimmed'"
          >{{ conflictCount }}</span>
        </div>
        <p
          v-if="!conflictCount"
          class="pt-1 text-[11px] text-dimmed"
        >
          No merge in progress.
        </p>
      </div>
    </div>
  </aside>
</template>
