<script setup lang="ts">
import type { GhIssue } from '~/stores/github'
import { openUrl } from '@tauri-apps/plugin-opener'

const git = useGitStore()
const github = useGithubStore()

const open = ref(false)
const loading = ref(false)

const issues = computed<GhIssue[]>(() =>
  git.selectedRepo ? github.issuesByRepo[git.selectedRepo] ?? [] : []
)

watch(open, async (v) => {
  if (v && git.selectedRepo) {
    loading.value = true
    await github.listIssues(git.selectedRepo)
    loading.value = false
  }
})

function labelColor(label: string) {
  // Simple hash-based color mapping for labels
  const colors = ['text-blue-400', 'text-green-400', 'text-purple-400', 'text-amber-400', 'text-red-400']
  let hash = 0
  for (const c of label) hash = (hash * 31 + c.charCodeAt(0)) & 0xffff
  return colors[hash % colors.length]
}
</script>

<template>
  <div class="panel-card overflow-hidden">
    <button
      class="flex items-center gap-1.5 w-full px-3 py-2 section-label hover:text-toned transition-colors"
      @click="open = !open"
    >
      <UIcon
        :name="open ? 'i-lucide-chevron-down' : 'i-lucide-chevron-right'"
        class="size-3"
      />
      <UIcon
        name="i-lucide-circle-dot"
        class="size-3"
      />
      Issues
      <span
        v-if="issues.length"
        class="font-mono text-dimmed"
      >{{ issues.length }}</span>
      <UIcon
        v-if="loading"
        name="i-lucide-loader-circle"
        class="size-3 animate-spin ml-auto"
      />
    </button>

    <div
      v-if="open"
      class="border-t border-(--ui-border-muted)"
    >
      <div
        v-if="issues.length"
        class="divide-y divide-(--ui-border-muted)"
      >
        <div
          v-for="issue in issues"
          :key="issue.number"
          class="group flex gap-2 px-3 py-2.5 hover:bg-elevated/40 cursor-pointer transition-colors"
          @click="openUrl(issue.html_url)"
        >
          <UIcon
            name="i-lucide-circle-dot"
            class="size-3.5 shrink-0 mt-0.5 text-green-500"
          />
          <div class="flex-1 min-w-0">
            <p class="text-[12px] text-toned leading-tight truncate">
              {{ issue.title }}
            </p>
            <div class="flex items-center gap-1.5 mt-1 flex-wrap">
              <span class="text-[10px] font-mono text-dimmed">#{{ issue.number }}</span>
              <span class="text-[10px] text-dimmed">{{ issue.author }}</span>
              <span
                v-for="label in issue.labels.slice(0, 3)"
                :key="label"
                class="text-[9.5px] px-1.5 py-0.5 rounded-full bg-accented"
                :class="labelColor(label)"
              >{{ label }}</span>
              <span
                v-if="issue.comments"
                class="text-[10px] text-dimmed flex items-center gap-0.5 ml-auto"
              >
                <UIcon
                  name="i-lucide-message-square"
                  class="size-2.5"
                />
                {{ issue.comments }}
              </span>
            </div>
          </div>
          <UIcon
            name="i-lucide-external-link"
            class="size-3 text-dimmed opacity-0 group-hover:opacity-100 mt-0.5 shrink-0"
          />
        </div>
      </div>
      <p
        v-else-if="!loading"
        class="px-3 py-3 text-xs text-dimmed italic"
      >
        No open issues.
      </p>
    </div>
  </div>
</template>
