<script setup lang="ts">
const ui = useUiStore()
const git = useGitStore()
const github = useGithubStore()
const workspaces = useWorkspacesStore()

// picking a file closes the PR detail (back to diff view)
watch(() => git.selected, (file) => {
  if (file) github.closePrDetail()
})

async function syncToWorkspace() {
  const repoPaths = workspaces.repos.map(r => r.path)

  // If selectedRepo doesn't belong to the current workspace, reset it
  if (git.selectedRepo && !repoPaths.includes(git.selectedRepo)) {
    git.resetSelection()
    github.closePrDetail()
  }

  if (!repoPaths.length) return

  if (!git.selectedRepo) {
    // Auto-select the first repo of this workspace
    await git.selectRepo(repoPaths[0]!)
  } else {
    await git.refresh(git.selectedRepo)
  }
  git.refreshAll(repoPaths)
}

// Sync when switching into File Changes view
watch(() => ui.view, (view) => {
  if (view === 'changes') syncToWorkspace()
})

// KEY FIX: sync when workspace changes (repos list updates after scan)
watch(() => workspaces.repos, (repos) => {
  // Only reset if we're actively on File Changes, or selectedRepo is stale
  const repoPaths = repos.map(r => r.path)
  if (git.selectedRepo && !repoPaths.includes(git.selectedRepo)) {
    git.resetSelection()
    github.closePrDetail()
  }
  // If the user is on File Changes view, also auto-select new workspace's repo
  if (ui.view === 'changes') syncToWorkspace()
}, { deep: false })

// Refresh on window focus (files often change externally)
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
  <div
    class="flex h-full min-h-0 min-w-0"
    style="min-width: 0;"
  >
    <GitRepoSidebar v-show="ui.leftSidebarOpen" />
    <GithubPrDetailView v-if="github.prDetail || github.loadingDetail" />
    <GitConflictHelper
      v-else-if="git.selected?.file.status === '!' && git.selected.repo"
      :path="git.selected.file.path"
      :repo="git.selected.repo"
    />
    <GitDiffViewer v-else />
    <GitCommitPanel v-show="ui.rightPanelOpen" />
    <GithubCiLogsModal />
  </div>
</template>
