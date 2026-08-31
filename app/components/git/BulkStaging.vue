<script setup lang="ts">
const git = useGitStore()

const folders = computed(() =>
  Object.entries(git.unstagedByFolder).sort(([a], [b]) => a.localeCompare(b))
)

const show = computed(() => {
  const total = git.status?.unstaged.length ?? 0
  return total >= 5 // only show bulk controls when there are many files
})

async function stageFolder(folder: string) {
  if (!git.selectedRepo) return
  await git.stageFolder(git.selectedRepo, folder)
}
</script>

<template>
  <div
    v-if="show"
    class="panel-card overflow-hidden"
  >
    <div class="flex items-center gap-1.5 px-3 py-2 section-label border-b border-(--ui-border-muted)">
      <UIcon
        name="i-lucide-folders"
        class="size-3"
      />
      Stage by Folder
    </div>
    <div class="px-1.5 py-1 space-y-0.5">
      <div
        v-for="[folder, files] in folders"
        :key="folder"
        class="group flex items-center gap-2 px-2 py-1.5 rounded-lg hover:bg-elevated/50 transition-colors"
      >
        <UIcon
          name="i-lucide-folder"
          class="size-3.5 text-blue-400 shrink-0"
        />
        <span class="flex-1 text-[12px] text-toned font-mono truncate">
          {{ folder === '.' ? 'root' : folder }}/
        </span>
        <span class="text-[10px] font-mono text-dimmed">{{ files.length }} files</span>
        <UButton
          label="Stage"
          size="xs"
          color="neutral"
          variant="ghost"
          class="opacity-0 group-hover:opacity-100 transition-opacity"
          @click="stageFolder(folder)"
        />
      </div>
    </div>
  </div>
</template>
