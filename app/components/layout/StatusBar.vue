<script setup lang="ts">
const terminals = useTerminalsStore()
const git = useGitStore()
const github = useGithubStore()

const branchInfo = computed(() => {
  const s = git.status
  if (!s) return null
  return {
    branch: s.branch ?? 'detached',
    ahead: s.ahead,
    behind: s.behind
  }
})
</script>

<template>
  <footer class="flex items-center h-6 shrink-0 px-3 gap-4 border-t border-default bg-muted text-[11px] text-muted select-none">
    <span class="flex items-center gap-1">
      <UIcon
        name="i-lucide-terminal"
        class="size-3"
      />
      {{ terminals.tabs.length }} session{{ terminals.tabs.length === 1 ? '' : 's' }}<template v-if="terminals.paneCount > terminals.tabs.length"> · {{ terminals.paneCount }} panes</template>
    </span>

    <span
      v-if="branchInfo"
      class="flex items-center gap-1"
    >
      <UIcon
        name="i-lucide-git-branch"
        class="size-3"
      />
      {{ branchInfo.branch }}
      <span
        v-if="branchInfo.ahead"
        class="text-green-500"
      >↑{{ branchInfo.ahead }}</span>
      <span
        v-if="branchInfo.behind"
        class="text-amber-400"
      >↓{{ branchInfo.behind }}</span>
    </span>

    <span class="ml-auto flex items-center gap-4">
      <span
        v-if="github.user"
        class="flex items-center gap-1"
      >
        <UIcon
          name="i-simple-icons-github"
          class="size-3"
        />
        {{ github.user.login }}
      </span>
      <span class="flex items-center gap-1">
        <span class="size-1.5 rounded-full bg-green-500" />
        Ready
      </span>
    </span>
  </footer>
</template>
