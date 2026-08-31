<script setup lang="ts">
const ui = useUiStore()
const workspaces = useWorkspacesStore()
const explorer = useExplorerStore()

const fileLines = computed(() => explorer.currentFile?.content.split('\n') ?? [])
const selectedLabel = computed(() => explorer.selectedPath || explorer.rootName)

function formatBytes(size: number | null) {
  if (size == null) return ''
  if (size < 1024) return `${size} B`
  if (size < 1024 * 1024) return `${(size / 1024).toFixed(1)} KB`
  return `${(size / 1024 / 1024).toFixed(1)} MB`
}

watch([
  () => ui.view,
  () => workspaces.active?.path
], ([view]) => {
  if (view === 'explorer') explorer.ensureRootLoaded()
}, { immediate: true })
</script>

<template>
  <div class="flex h-full min-h-0 min-w-0 bg-default">
    <aside
      v-show="ui.leftSidebarOpen"
      class="flex w-72 shrink-0 flex-col border-r border-default bg-muted/40"
    >
      <div class="flex h-12 shrink-0 items-center gap-2 border-b border-default px-3">
        <UIcon
          name="i-lucide-folder-tree"
          class="size-4 text-sky-400"
        />
        <div class="min-w-0 flex-1">
          <p class="truncate text-[12.5px] font-semibold text-highlighted">
            {{ explorer.rootName }}
          </p>
          <p class="truncate text-[10.5px] text-dimmed">
            Explorer
          </p>
        </div>
        <UTooltip text="Refresh explorer">
          <button
            class="flex size-7 items-center justify-center rounded-md text-dimmed transition-colors hover:bg-elevated/50 hover:text-toned"
            aria-label="Refresh explorer"
            :disabled="!explorer.rootPath"
            @click="explorer.refresh"
          >
            <UIcon
              name="i-lucide-refresh-cw"
              class="size-3.5"
            />
          </button>
        </UTooltip>
      </div>

      <div class="min-h-0 flex-1 overflow-auto p-2">
        <div
          v-if="!explorer.rootPath"
          class="px-2 py-4 text-xs text-dimmed"
        >
          Add a workspace to browse files.
        </div>
        <template v-else-if="explorer.rootEntries.length">
          <ExplorerFileTreeNode
            v-for="entry in explorer.rootEntries"
            :key="entry.path"
            :entry="entry"
            :level="0"
          />
        </template>
        <div
          v-else-if="explorer.isLoading('')"
          class="flex items-center gap-2 px-2 py-4 text-xs text-dimmed"
        >
          <UIcon
            name="i-lucide-loader-2"
            class="size-3 animate-spin"
          />
          Loading files...
        </div>
        <div
          v-else
          class="px-2 py-4 text-xs text-dimmed"
        >
          No files in this workspace.
        </div>
      </div>
    </aside>

    <section class="flex min-w-0 flex-1 flex-col">
      <div class="flex h-12 shrink-0 items-center gap-2 border-b border-default px-4">
        <UIcon
          name="i-lucide-file"
          class="size-4 text-dimmed"
        />
        <span class="min-w-0 flex-1 truncate text-[12.5px] font-medium text-toned">
          {{ selectedLabel }}
        </span>
        <span
          v-if="explorer.currentFile"
          class="shrink-0 text-[10.5px] font-mono text-dimmed"
        >
          {{ formatBytes(explorer.currentFile.size) }}
        </span>
      </div>

      <div class="min-h-0 flex-1 overflow-auto">
        <div
          v-if="explorer.loadingFile"
          class="flex h-full items-center justify-center gap-2 text-xs text-dimmed"
        >
          <UIcon
            name="i-lucide-loader-2"
            class="size-4 animate-spin"
          />
          Opening file...
        </div>
        <div
          v-else-if="explorer.error"
          class="flex h-full items-center justify-center px-6 text-center text-xs text-red-300"
        >
          {{ explorer.error }}
        </div>
        <div
          v-else-if="explorer.currentFile"
          class="min-h-full bg-default"
        >
          <div
            v-if="explorer.currentFile.truncated"
            class="border-b border-amber-500/20 bg-amber-500/10 px-4 py-2 text-xs text-amber-200"
          >
            File is large, showing the first chunk only.
          </div>
          <pre class="m-0 min-h-full overflow-visible p-4 font-mono text-[12px] leading-5 text-toned"><code>{{ fileLines.join('\n') }}</code></pre>
        </div>
        <div
          v-else
          class="flex h-full flex-col items-center justify-center gap-3 text-dimmed"
        >
          <UIcon
            name="i-lucide-files"
            class="size-10 opacity-45"
          />
          <p class="text-xs">
            Select a file to preview.
          </p>
        </div>
      </div>
    </section>
  </div>
</template>
