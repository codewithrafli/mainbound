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

// …and when the app window regains focus (files often change in between)
let unlistenFocus: (() => void) | undefined
onMounted(async () => {
  const { getCurrentWindow } = await import('@tauri-apps/api/window')
  unlistenFocus = await getCurrentWindow().onFocusChanged(({ payload: focused }) => {
    if (focused && ui.view === 'changes' && git.selectedRepo) {
      git.refresh(git.selectedRepo)
    }
  })
})
onBeforeUnmount(() => unlistenFocus?.())
</script>

<template>
  <div class="flex h-full min-h-0">
    <GitRepoSidebar />
    <GitDiffViewer />
    <GitCommitPanel />
  </div>
</template>
