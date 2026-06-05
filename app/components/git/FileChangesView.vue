<script setup lang="ts">
const ui = useUiStore()
const git = useGitStore()
const workspaces = useWorkspacesStore()

// Refresh statuses whenever the user switches into this view
watch(() => ui.view, async (view) => {
  if (view !== 'changes') return
  const repoPaths = workspaces.repos.map(r => r.path)
  if (!git.selectedRepo && repoPaths.length) {
    await git.selectRepo(repoPaths[0]!)
  } else if (git.selectedRepo) {
    await git.refresh(git.selectedRepo)
  }
  git.refreshAll(repoPaths)
})
</script>

<template>
  <div class="flex h-full min-h-0">
    <GitRepoSidebar />
    <GitDiffViewer />
    <GitCommitPanel />
  </div>
</template>
