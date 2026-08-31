<script setup lang="ts">
import type { ExplorerEntry } from '~/stores/explorer'

defineOptions({ name: 'ExplorerFileTreeNode' })

const props = defineProps<{
  entry: ExplorerEntry
  level: number
}>()

const explorer = useExplorerStore()

const isDir = computed(() => props.entry.kind === 'dir')
const expanded = computed(() => explorer.isExpanded(props.entry.path))
const loading = computed(() => explorer.isLoading(props.entry.path))
const children = computed(() => explorer.childrenByPath[props.entry.path] ?? [])
const loaded = computed(() => explorer.childrenByPath[props.entry.path] !== undefined)
const active = computed(() => explorer.selectedPath === props.entry.path)
const indent = computed(() => `${8 + props.level * 14}px`)

const icon = computed(() => {
  if (isDir.value) return expanded.value ? 'i-lucide-folder-open' : 'i-lucide-folder'
  const ext = props.entry.name.split('.').pop()?.toLowerCase()
  if (['vue', 'ts', 'tsx', 'js', 'jsx', 'rs', 'go', 'php', 'py', 'json', 'css', 'scss', 'html'].includes(ext ?? '')) {
    return 'i-lucide-file-code-2'
  }
  if (['md', 'txt', 'yml', 'yaml', 'toml', 'env', 'lock'].includes(ext ?? '')) {
    return 'i-lucide-file-text'
  }
  return 'i-lucide-file'
})

function select() {
  if (isDir.value) {
    explorer.toggleDir(props.entry)
  } else {
    explorer.selectFile(props.entry)
  }
}
</script>

<template>
  <div>
    <button
      class="group flex h-7 w-full min-w-0 items-center gap-1.5 rounded-md pr-2 text-left text-[12.5px] transition-colors"
      :class="active
        ? 'bg-elevated text-highlighted ring-1 ring-(--ui-border-accented)'
        : 'text-muted hover:bg-elevated/45 hover:text-toned'"
      :style="{ paddingLeft: indent }"
      @click="select"
    >
      <UIcon
        v-if="isDir"
        :name="expanded ? 'i-lucide-chevron-down' : 'i-lucide-chevron-right'"
        class="size-3 shrink-0 text-dimmed"
      />
      <span
        v-else
        class="w-3 shrink-0"
      />
      <UIcon
        :name="icon"
        class="size-3.5 shrink-0"
        :class="isDir ? 'text-sky-400' : 'text-dimmed group-hover:text-toned'"
      />
      <span class="min-w-0 flex-1 truncate">{{ entry.name }}</span>
      <UIcon
        v-if="loading"
        name="i-lucide-loader-2"
        class="size-3 shrink-0 animate-spin text-dimmed"
      />
    </button>

    <div v-if="isDir && expanded">
      <template v-if="children.length">
        <ExplorerFileTreeNode
          v-for="child in children"
          :key="child.path"
          :entry="child"
          :level="level + 1"
        />
      </template>
      <p
        v-else-if="loaded && !loading"
        class="h-6 truncate px-2 text-[11px] leading-6 text-dimmed"
        :style="{ paddingLeft: `${22 + (level + 1) * 14}px` }"
      >
        Empty folder
      </p>
    </div>
  </div>
</template>
