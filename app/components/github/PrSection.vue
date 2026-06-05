<script setup lang="ts">
const git = useGitStore()
const github = useGithubStore()

const prsOpen = ref(true)
const createOpen = ref(false)

const title = ref('')
const body = ref('')
const base = ref('main')
const creating = ref(false)

const repo = computed(() => git.selectedRepo)
const prs = computed(() => (repo.value ? github.prsByRepo[repo.value] ?? [] : []))

watch([repo, () => github.user], ([r, u]) => {
  if (r && u) github.listPrs(r)
}, { immediate: true })

function ciColor(sha: string): string {
  const c = github.checksBySha[sha]
  if (!c || !c.total) return 'bg-neutral-600'
  if (c.failed) return 'bg-red-500'
  if (c.pending) return 'bg-amber-400'
  return 'bg-green-500'
}

async function openPr(url: string) {
  const { openUrl } = await import('@tauri-apps/plugin-opener')
  await openUrl(url)
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
    openPr(pr.html_url)
  }
}
</script>

<template>
  <div v-if="github.user && repo">
    <button
      class="flex items-center gap-1.5 w-full px-3 pt-4 pb-1.5 text-[10px] font-medium tracking-wider text-dimmed uppercase hover:text-muted"
      @click="prsOpen = !prsOpen"
    >
      <UIcon
        :name="prsOpen ? 'i-lucide-chevron-down' : 'i-lucide-chevron-right'"
        class="size-3"
      />
      Pull Requests
      <UBadge
        color="neutral"
        variant="soft"
        size="sm"
      >
        {{ prs.length }}
      </UBadge>
      <UButton
        label="New"
        icon="i-lucide-git-pull-request-create"
        color="neutral"
        variant="link"
        size="xs"
        class="ml-auto"
        @click.stop="createOpen = true"
      />
    </button>

    <div
      v-show="prsOpen"
      class="px-3 pb-2 space-y-2"
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
        @click="openPr(pr.html_url)"
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
          name="i-lucide-external-link"
          class="ml-auto mt-0.5 size-3 shrink-0 text-dimmed opacity-0 group-hover:opacity-100"
        />
      </button>
      <p
        v-if="!github.loadingPrs && !prs.length"
        class="text-[11px] text-dimmed italic"
      >
        No open pull requests.
      </p>
      <UAlert
        v-if="github.error"
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
            label="Create Pull Request"
            color="primary"
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
