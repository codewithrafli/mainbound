<script setup lang="ts">
import { DiffView, DiffModeEnum } from '@git-diff-view/vue'
import '@git-diff-view/vue/styles/diff-view.css'

const git = useGitStore()

const LANGS: Record<string, string> = {
  ts: 'typescript', tsx: 'tsx', js: 'javascript', jsx: 'jsx', mjs: 'javascript',
  vue: 'vue', css: 'css', scss: 'scss', html: 'html', json: 'json',
  md: 'markdown', rs: 'rust', py: 'python', go: 'go', sh: 'bash',
  yml: 'yaml', yaml: 'yaml', toml: 'ini', sql: 'sql'
}

function langOf(path: string): string {
  const ext = path.split('.').pop()?.toLowerCase() ?? ''
  return LANGS[ext] ?? 'plaintext'
}

const data = computed(() => {
  if (!git.selected || !git.diff.trim()) return null
  const path = git.selected.file.path
  return {
    oldFile: { fileName: git.selected.file.orig_path ?? path, fileLang: langOf(path) },
    newFile: { fileName: path, fileLang: langOf(path) },
    hunks: [git.diff]
  }
})
</script>

<template>
  <section class="flex-1 min-w-0 overflow-auto bg-default">
    <template v-if="git.selected">
      <div class="sticky top-0 z-10 flex items-center gap-2 px-4 py-2 border-b border-default bg-muted/95 backdrop-blur text-xs">
        <GitStatusBadge :status="git.selected.file.status" />
        <span class="font-mono text-toned truncate">{{ git.selected.file.path }}</span>
        <UBadge
          v-if="git.selected.file.staged"
          color="success"
          variant="soft"
          size="sm"
        >
          staged
        </UBadge>
        <span
          v-if="git.selected.file.added !== null"
          class="ml-auto font-mono text-[10px] text-green-500"
        >+{{ git.selected.file.added }}</span>
        <span
          v-if="git.selected.file.removed !== null"
          class="font-mono text-[10px] text-red-500"
        >-{{ git.selected.file.removed }}</span>
      </div>

      <div
        v-if="git.diffLoading"
        class="flex items-center justify-center py-16"
      >
        <UIcon
          name="i-lucide-loader-circle"
          class="size-5 animate-spin text-dimmed"
        />
      </div>

      <!-- Conflict resolution helper replaces diff for conflict files -->
      <GitConflictHelper
        v-else-if="git.selected?.file.status === '!'"
        :path="git.selected.file.path"
        :repo="git.selected.repo"
      />

      <DiffView
        v-else-if="data"
        :data="data"
        :diff-view-mode="DiffModeEnum.Unified"
        :diff-view-theme="'dark'"
        :diff-view-highlight="true"
        :diff-view-wrap="false"
        :diff-view-font-size="12"
      />

      <div
        v-else
        class="flex items-center justify-center py-16 text-sm text-dimmed"
      >
        No textual diff (binary file or empty change).
      </div>
    </template>

    <div
      v-else
      class="flex h-full items-center justify-center"
    >
      <div class="panel-card flex flex-col items-center gap-3 px-12 py-12 rounded-2xl">
        <div class="flex items-center justify-center size-10 rounded-xl bg-elevated border border-default">
          <UIcon
            name="i-lucide-git-branch"
            class="size-5 text-dimmed"
          />
        </div>
        <h2 class="text-base font-medium text-highlighted">
          File Changes
        </h2>
        <p class="text-sm text-muted">
          Pick a file from the left sidebar to see its diff here.
        </p>
      </div>
    </div>
  </section>
</template>
