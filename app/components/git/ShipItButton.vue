<script setup lang="ts">
const git = useGitStore()
const github = useGithubStore()
const { settings } = storeToRefs(useSettingsStore())
const toast = useToast()

const hasChanges = computed(() =>
  (git.status?.unstaged.length ?? 0) + (git.status?.staged.length ?? 0) > 0
)

const stepLabel: Record<string, string> = {
  staging: 'Staging…',
  generating: 'Writing message…',
  committing: 'Committing…',
  pushing: 'Pushing…',
  done: 'Shipped'
}

const busy = computed(() => !!git.shipStep && git.shipStep !== 'done')

async function ship() {
  if (!git.selectedRepo || git.shipStep) return
  const repo = git.selectedRepo
  const ok = await git.shipIt(repo, settings.value.aiProvider, async () => {
    await github.push(repo, { autoDraft: settings.value.autoDraftPr })
  })
  if (ok) {
    toast.add({
      title: 'Shipped',
      description: settings.value.autoDraftPr ? 'Committed, pushed, draft PR created.' : 'Committed and pushed.',
      icon: 'i-lucide-rocket',
      color: 'success'
    })
  } else if (git.shipError) {
    toast.add({ title: 'Ship failed', description: git.shipError, color: 'error', icon: 'i-lucide-x-circle' })
  }
}
</script>

<template>
  <div class="space-y-1.5">
    <button
      class="group relative flex w-full items-center justify-center gap-2 rounded-lg py-2 text-[13px] font-medium
             text-white transition-all disabled:opacity-40 disabled:cursor-not-allowed
             bg-gradient-to-r from-violet-600 to-fuchsia-600 hover:from-violet-500 hover:to-fuchsia-500
             shadow-sm hover:shadow-md hover:shadow-fuchsia-500/20"
      :disabled="!hasChanges || busy || git.gitBusy"
      @click="ship"
    >
      <UIcon
        :name="busy ? 'i-lucide-loader-circle' : 'i-lucide-rocket'"
        class="size-3.5"
        :class="busy ? 'animate-spin' : 'group-hover:-translate-y-0.5 transition-transform'"
      />
      {{ git.shipStep ? stepLabel[git.shipStep] : 'Ship It' }}
    </button>

    <div class="flex items-center justify-between px-0.5">
      <label class="flex items-center gap-1.5 cursor-pointer select-none">
        <input
          v-model="settings.autoDraftPr"
          type="checkbox"
          class="size-3 rounded accent-violet-500"
        >
        <span class="text-[10.5px] text-dimmed">Auto draft PR</span>
      </label>
      <UTooltip
        text="Stage all → AI commit → Commit → Push"
        :content="{ side: 'top' }"
      >
        <span class="text-[10px] text-dimmed">stage · commit · push</span>
      </UTooltip>
    </div>
  </div>
</template>
