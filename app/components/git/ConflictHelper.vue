<script setup lang="ts">
const props = defineProps<{
  path: string
  repo: string
}>()

const git = useGitStore()
const toast = useToast()
const resolving = ref<'ours' | 'theirs' | null>(null)

async function resolve(side: 'ours' | 'theirs') {
  resolving.value = side
  const ok = await git.conflictResolve(props.repo, props.path, side)
  resolving.value = null
  if (ok) toast.add({ title: `Resolved with ${side === 'ours' ? 'our' : 'their'} changes`, icon: 'i-lucide-check-circle' })
}
</script>

<template>
  <div
    class="flex-1 min-w-0 min-h-0 flex flex-col gap-3 p-4 overflow-auto"
    style="min-width: 0; min-height: 0;"
  >
    <div class="flex items-center gap-2">
      <UIcon
        name="i-lucide-triangle-alert"
        class="size-4 text-orange-400"
      />
      <span class="text-sm font-medium text-highlighted">Merge conflict</span>
      <span class="font-mono text-xs text-dimmed truncate">{{ path }}</span>
    </div>

    <p class="text-xs text-muted">
      This file has unresolved merge conflicts. Choose how to resolve:
    </p>

    <div class="grid grid-cols-2 gap-3">
      <!-- Accept Ours -->
      <div class="panel-card p-3 space-y-2">
        <div class="flex items-center gap-1.5 section-label">
          <UIcon
            name="i-lucide-arrow-left"
            class="size-3 text-blue-400"
          />
          Accept Ours (HEAD)
        </div>
        <p class="text-[11px] text-dimmed">
          Keep your local changes, discard incoming.
        </p>
        <UButton
          label="Accept Ours"
          color="neutral"
          variant="solid"
          size="xs"
          block
          :loading="resolving === 'ours'"
          :disabled="resolving !== null"
          @click="resolve('ours')"
        />
      </div>

      <!-- Accept Theirs -->
      <div class="panel-card p-3 space-y-2">
        <div class="flex items-center gap-1.5 section-label">
          <UIcon
            name="i-lucide-arrow-right"
            class="size-3 text-purple-400"
          />
          Accept Theirs
        </div>
        <p class="text-[11px] text-dimmed">
          Keep incoming changes, discard local.
        </p>
        <UButton
          label="Accept Theirs"
          color="neutral"
          variant="outline"
          size="xs"
          block
          :loading="resolving === 'theirs'"
          :disabled="resolving !== null"
          @click="resolve('theirs')"
        />
      </div>
    </div>

    <div class="panel-card p-3 text-[11px] text-muted space-y-1">
      <p>
        <span class="font-mono text-blue-400">&lt;&lt;&lt;&lt;&lt;&lt;&lt; HEAD</span> = your changes
      </p>
      <p>
        <span class="font-mono text-dimmed">=======</span> = separator
      </p>
      <p>
        <span class="font-mono text-purple-400">&gt;&gt;&gt;&gt;&gt;&gt;&gt;</span> = incoming changes
      </p>
      <p class="pt-1 text-dimmed">
        For complex conflicts, resolve manually in your editor then stage the file.
      </p>
    </div>
  </div>
</template>
