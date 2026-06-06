<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'

const git = useGitStore()
const github = useGithubStore()
const { settings } = storeToRefs(useSettingsStore())

async function doPush() {
  if (!git.selectedRepo) return
  if (await github.push(git.selectedRepo)) git.refresh(git.selectedRepo)
}

async function doPull() {
  if (!git.selectedRepo) return
  if (await github.pull(git.selectedRepo)) git.refresh(git.selectedRepo)
}

const summary = ref('')
const description = ref('')

const canCommit = computed(() =>
  !!summary.value.trim() && !!git.status?.staged.length && !git.committing
)

async function doCommit() {
  if (!canCommit.value || !git.selectedRepo) return
  const ok = await git.commit(git.selectedRepo, summary.value.trim(), description.value.trim())
  if (ok) {
    summary.value = ''
    description.value = ''
  }
}

const generating = ref(false)
const aiError = ref<string | null>(null)

async function generateWithAi() {
  if (!git.selectedRepo || !git.status?.staged.length || generating.value) return
  generating.value = true
  aiError.value = null
  try {
    const suggestion = await invoke<{ summary: string, description: string }>(
      'ai_commit_message',
      { repo: git.selectedRepo, provider: settings.value.aiProvider }
    )
    summary.value = suggestion.summary
    description.value = suggestion.description
  } catch (e) {
    aiError.value = String(e)
  } finally {
    generating.value = false
  }
}
</script>

<template>
  <aside class="flex flex-col w-72 shrink-0 border-l border-default bg-muted/40 overflow-y-auto">
    <template v-if="git.status">
      <div class="p-2 space-y-2">
        <!-- branch card -->
        <div class="panel-card px-3 py-2.5">
          <div class="flex items-center gap-2">
            <UIcon
              name="i-lucide-git-branch"
              class="size-3.5 text-muted"
            />
            <span class="text-[13px] font-medium text-highlighted truncate">{{ git.status.branch ?? 'detached' }}</span>
            <span
              v-if="git.status.oid"
              class="text-[10px] font-mono text-dimmed"
            >{{ git.status.oid }}</span>
          </div>
          <div class="flex items-center gap-2 pt-1.5 text-[10.5px] font-mono">
            <span
              v-if="git.status.ahead"
              class="text-green-500"
              title="Commits ahead of upstream"
            >↑{{ git.status.ahead }}</span>
            <span
              v-if="git.status.behind"
              class="text-amber-400"
              title="Commits behind upstream"
            >↓{{ git.status.behind }}</span>
            <span
              v-if="!git.status.ahead && !git.status.behind"
              class="text-dimmed"
            >synced with upstream</span>
            <span class="ml-auto flex items-center">
              <UButton
                icon="i-lucide-arrow-down-to-line"
                color="neutral"
                variant="ghost"
                size="xs"
                aria-label="Pull"
                :loading="github.syncing === 'pull'"
                @click="doPull"
              />
              <UButton
                icon="i-lucide-arrow-up-from-line"
                color="neutral"
                variant="ghost"
                size="xs"
                aria-label="Push"
                :loading="github.syncing === 'push'"
                @click="doPush"
              />
              <UButton
                icon="i-lucide-refresh-cw"
                color="neutral"
                variant="ghost"
                size="xs"
                aria-label="Refresh"
                @click="git.refresh(git.selectedRepo!)"
              />
            </span>
          </div>
        </div>

        <!-- staged card -->
        <div class="panel-card overflow-hidden">
          <div class="flex items-center gap-1.5 px-3 py-2 section-label border-b border-(--ui-border-muted)">
            <UIcon
              name="i-lucide-layers"
              class="size-3"
            />
            Staged
            <span class="font-mono text-dimmed">{{ git.status.staged.length }}</span>
            <UButton
              label="Stage all"
              color="neutral"
              variant="link"
              size="xs"
              class="ml-auto"
              :disabled="!git.status.unstaged.length"
              @click="git.stageAll(git.selectedRepo!)"
            />
          </div>
          <div class="px-1.5 py-1 space-y-0.5">
            <GitChangeRow
              v-for="file in git.status.staged"
              :key="`s-${file.path}`"
              :file="file"
              :active="git.selected?.file.path === file.path && git.selected?.file.staged"
              @select="git.selectFile(git.selectedRepo!, file)"
              @action="git.unstage(git.selectedRepo!, [file.path])"
            />
            <p
              v-if="!git.status.staged.length"
              class="px-2 py-2 text-[11px] text-dimmed italic"
            >
              Nothing staged yet.
            </p>
          </div>
        </div>

        <!-- commit card -->
        <div class="panel-card overflow-hidden">
          <div class="flex items-center gap-1.5 px-3 py-2 section-label border-b border-(--ui-border-muted)">
            <UIcon
              name="i-lucide-git-commit-horizontal"
              class="size-3"
            />
            Commit
          </div>
          <div class="p-2.5 space-y-2">
            <UInput
              v-model="summary"
              placeholder="Summary (required)"
              size="sm"
              class="w-full"
              @keydown.meta.enter="doCommit"
            />
            <UTextarea
              v-model="description"
              placeholder="Description (optional)"
              :rows="3"
              size="sm"
              class="w-full"
              @keydown.meta.enter="doCommit"
            />
            <div class="flex items-center justify-between">
              <UButton
                :label="generating ? 'Generating…' : 'Generate with AI'"
                icon="i-lucide-sparkles"
                color="neutral"
                variant="link"
                size="xs"
                :loading="generating"
                :disabled="!git.status.staged.length || generating"
                @click="generateWithAi"
              />
              <span class="text-[10px] text-dimmed">{{ settings.aiProvider === 'codex' ? 'Codex' : 'Claude' }} · ⌘↵</span>
            </div>
            <UAlert
              v-if="aiError"
              color="error"
              variant="soft"
              :description="aiError"
              class="text-xs"
            />
            <!-- white primary CTA -->
            <UButton
              label="Commit"
              color="neutral"
              variant="solid"
              size="sm"
              block
              :loading="git.committing"
              :disabled="!canCommit"
              @click="doCommit"
            />
            <UAlert
              v-if="git.error"
              color="error"
              variant="soft"
              :description="git.error"
              class="text-xs"
            />
          </div>
        </div>

        <!-- history card -->
        <div class="panel-card overflow-hidden">
          <div class="flex items-center gap-1.5 px-3 py-2 section-label border-b border-(--ui-border-muted)">
            <UIcon
              name="i-lucide-history"
              class="size-3"
            />
            History
            <span class="font-mono text-dimmed">{{ git.log.length }}</span>
          </div>
          <div class="px-3 py-2.5 space-y-2.5">
            <div
              v-for="commit in git.log"
              :key="commit.hash"
              class="flex gap-2"
            >
              <span class="mt-1 size-1.5 shrink-0 rounded-full bg-blue-500" />
              <div class="min-w-0">
                <p class="text-[12px] text-toned truncate leading-tight">
                  {{ commit.subject }}
                </p>
                <p class="text-[10px] text-dimmed font-mono leading-tight pt-0.5">
                  {{ commit.short_hash }} · {{ commit.author }} · {{ useRelativeDate(commit.date) }}
                </p>
              </div>
            </div>
            <p
              v-if="!git.log.length"
              class="text-[11px] text-dimmed italic"
            >
              No commits yet.
            </p>
          </div>
        </div>

        <!-- pull requests -->
        <GithubPrSection />
      </div>
    </template>

    <div
      v-else
      class="flex flex-1 items-center justify-center p-6"
    >
      <p class="text-xs text-dimmed text-center">
        Select a repository to stage and commit changes.
      </p>
    </div>
  </aside>
</template>
