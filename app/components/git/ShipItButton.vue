<script setup lang="ts">
const git = useGitStore()
const github = useGithubStore()
const { settings } = storeToRefs(useSettingsStore())
const toast = useToast()

const hasChanges = computed(() =>
  (git.status?.unstaged.length ?? 0) + (git.status?.staged.length ?? 0) > 0
)

const stepLabel: Record<string, string> = {
  staging:    '⟳ Staging all…',
  generating: '✨ Generating message…',
  committing: '⟳ Committing…',
  pushing:    '⟳ Pushing…',
  done:       '✓ Shipped!'
}

async function ship() {
  if (!git.selectedRepo || git.shipStep) return
  const repo = git.selectedRepo
  const ok = await git.shipIt(repo, settings.value.aiProvider, async () => {
    await github.push(repo, { autoDraft: settings.value.autoDraftPr })
  })
  if (ok) {
    toast.add({
      title: 'Shipped! 🚀',
      description: settings.value.autoDraftPr ? 'Committed, pushed, and draft PR created.' : 'Committed and pushed.',
      icon: 'i-lucide-rocket',
      color: 'success'
    })
  } else if (git.shipError) {
    toast.add({ title: 'Ship failed', description: git.shipError, color: 'error', icon: 'i-lucide-x-circle' })
  }
}
</script>

<template>
  <div class="panel-card overflow-hidden">
    <div class="px-3 py-2.5 space-y-2">
      <div class="flex items-center gap-1.5 section-label">
        <UIcon name="i-lucide-rocket" class="size-3" />
        Ship It
        <UTooltip
          text="Stage all → AI commit message → Commit → Push"
          :content="{ side: 'top' }"
        >
          <UIcon name="i-lucide-info" class="size-3 text-dimmed ml-auto" />
        </UTooltip>
      </div>

      <p v-if="git.shipStep" class="text-[11px] font-mono text-amber-400 animate-pulse">
        {{ stepLabel[git.shipStep] }}
      </p>

      <UButton
        :label="git.shipStep ? stepLabel[git.shipStep]! : 'Ship It 🚀'"
        class="w-full btn-gradient"
        :disabled="!hasChanges || !!git.shipStep || git.gitBusy"
        :loading="!!git.shipStep && git.shipStep !== 'done'"
        @click="ship"
      />

      <label class="flex items-center gap-2 cursor-pointer select-none">
        <input
          v-model="settings.autoDraftPr"
          type="checkbox"
          class="size-3 accent-purple-500"
        >
        <span class="text-[10.5px] text-muted">Auto-create draft PR on push</span>
      </label>
    </div>
  </div>
</template>
