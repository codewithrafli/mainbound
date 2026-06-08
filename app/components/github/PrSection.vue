<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'

const git = useGitStore()
const github = useGithubStore()
const { settings } = storeToRefs(useSettingsStore())

const prsOpen = ref(true)
const createOpen = ref(false)

const title = ref('')
const body = ref('')
const base = ref('main')
const creating = ref(false)

const repo = computed(() => git.selectedRepo)
const prs = computed(() => (repo.value ? github.prsByRepo[repo.value] ?? [] : []))

watch([repo, () => github.user], ([r, u]) => {
  // Clear stale error when switching repos
  github.error = null
  if (r && u) github.listPrs(r)
}, { immediate: true })

// Auto-fill PR template when create modal opens
watch(createOpen, async (v) => {
  if (v && repo.value && !body.value.trim()) {
    await git.loadPrTemplate(repo.value)
    if (git.prTemplate) body.value = git.prTemplate
  }
})

function ciColor(sha: string): string {
  const c = github.checksBySha[sha]
  if (!c || !c.total) return 'bg-neutral-600'
  if (c.failed) return 'bg-red-500'
  if (c.pending) return 'bg-amber-400'
  return 'bg-green-500'
}

const generating = ref(false)
const aiError = ref<string | null>(null)

async function generateWithAi() {
  if (!repo.value || generating.value) return
  generating.value = true
  aiError.value = null
  try {
    const suggestion = await invoke<{ title: string, body: string }>('ai_pr_message', {
      repo: repo.value,
      base: base.value.trim() || 'main',
      provider: settings.value.aiProvider
    })
    title.value = suggestion.title
    body.value = suggestion.body
  } catch (e) {
    aiError.value = String(e)
  } finally {
    generating.value = false
  }
}

async function submitPr() {
  if (!repo.value || !title.value.trim() || !git.status?.branch) return
  creating.value = true
  const pr = await github.createPr(
    repo.value,
    git.status.branch,
    base.value.trim() || 'main',
    title.value.trim(),
    body.value.trim()
  )
  creating.value = false
  if (pr) {
    title.value = ''
    body.value = ''
    createOpen.value = false
    // open the new PR in-app
    github.openPrDetail(repo.value, pr.number)
  }
}
</script>

<template>
  <div
    v-if="github.user && repo"
    class="panel-card overflow-hidden"
  >
    <div class="flex items-center gap-1.5 px-3 py-2 section-label border-b border-(--ui-border-muted)">
      <button
        class="flex items-center gap-1.5 hover:text-toned"
        @click="prsOpen = !prsOpen"
      >
        <UIcon
          :name="prsOpen ? 'i-lucide-chevron-down' : 'i-lucide-chevron-right'"
          class="size-3"
        />
        Pull Requests
        <span class="font-mono text-dimmed">{{ prs.length }}</span>
      </button>
      <UButton
        label="New"
        icon="i-lucide-git-pull-request-create"
        color="neutral"
        variant="link"
        size="xs"
        class="ml-auto"
        @click.stop="createOpen = true"
      />
    </div>

    <div
      v-show="prsOpen"
      class="px-3 py-2.5 space-y-2"
    >
      <div
        v-if="github.loadingPrs"
        class="flex justify-center py-2"
      >
        <UIcon
          name="i-lucide-loader-circle"
          class="size-4 animate-spin text-dimmed"
        />
      </div>
      <button
        v-for="pr in prs"
        :key="pr.number"
        class="flex w-full gap-2 text-left group"
        @click="github.openPrDetail(repo!, pr.number)"
      >
        <span
          class="mt-1 size-1.5 shrink-0 rounded-full"
          :class="ciColor(pr.head_sha)"
        />
        <span class="min-w-0">
          <span class="block text-[12px] text-toned truncate leading-tight group-hover:text-highlighted">
            <span
              v-if="pr.draft"
              class="text-dimmed"
            >[draft]</span> {{ pr.title }}
          </span>
          <span class="block text-[10px] text-dimmed font-mono leading-tight pt-0.5 truncate">
            #{{ pr.number }} · {{ pr.head_ref }} → {{ pr.base_ref }} · {{ pr.author }}
          </span>
        </span>
        <UIcon
          name="i-lucide-panel-right-open"
          class="ml-auto mt-0.5 size-3 shrink-0 text-dimmed opacity-0 group-hover:opacity-100"
        />
      </button>
      <p
        v-if="!github.loadingPrs && !prs.length"
        class="text-[11px] text-dimmed italic"
      >
        No open pull requests.
      </p>
      <!-- Only show errors that are NOT about missing remote — that's
           handled by hiding the whole section (v-if="github.user && repo") -->
      <UAlert
        v-if="github.error && !github.error.includes('remote')"
        color="error"
        variant="soft"
        :description="github.error"
        class="text-xs"
      />
    </div>

    <!-- Create PR modal -->
    <UModal
      v-model:open="createOpen"
      title="Create Pull Request"
      :ui="{ content: 'max-w-md' }"
    >
      <template #body>
        <div class="space-y-3">
          <div class="flex items-center gap-2 text-xs text-muted">
            <UIcon
              name="i-lucide-git-branch"
              class="size-3.5"
            />
            <code class="text-toned">{{ git.status?.branch ?? '?' }}</code>
            <UIcon
              name="i-lucide-arrow-right"
              class="size-3"
            />
            <UInput
              v-model="base"
              size="xs"
              class="w-28"
              placeholder="base"
            />
          </div>
          <UInput
            v-model="title"
            placeholder="Title (required)"
            size="sm"
            class="w-full"
          />
          <UTextarea
            v-model="body"
            placeholder="Description (optional)"
            :rows="4"
            size="sm"
            class="w-full"
          />
          <UButton
            :label="generating ? 'Generating…' : 'Generate with AI'"
            icon="i-lucide-sparkles"
            color="neutral"
            variant="link"
            size="xs"
            :loading="generating"
            @click="generateWithAi"
          />
          <UAlert
            v-if="aiError"
            color="error"
            variant="soft"
            :description="aiError"
            class="text-xs"
          />
          <UButton
            label="Create Pull Request"
            color="neutral"
            variant="solid"
            size="sm"
            block
            :loading="creating"
            :disabled="!title.trim()"
            @click="submitPr"
          />
        </div>
      </template>
    </UModal>
  </div>
</template>
