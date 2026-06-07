<script setup lang="ts">
const git = useGitStore()
const toast = useToast()

const open = ref(false)
const pushMessage = ref('')
const pushing = ref(false)

async function doStashPush() {
  if (!git.selectedRepo) return
  pushing.value = true
  const ok = await git.stashPush(git.selectedRepo, pushMessage.value || undefined)
  pushing.value = false
  if (ok) {
    pushMessage.value = ''
    toast.add({ title: 'Stashed', icon: 'i-lucide-package' })
  }
}

async function doApply(index: number) {
  if (!git.selectedRepo) return
  const ok = await git.stashApply(git.selectedRepo, index)
  if (ok) toast.add({ title: `Stash @{${index}} applied`, icon: 'i-lucide-package-open' })
}

async function doDrop(index: number) {
  if (!git.selectedRepo) return
  const ok = await git.stashDrop(git.selectedRepo, index)
  if (ok) toast.add({ title: `Stash @{${index}} dropped`, icon: 'i-lucide-trash-2' })
}

watch(open, async (v) => {
  if (v && git.selectedRepo) await git.refreshStash(git.selectedRepo)
})
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
        name="i-lucide-package"
        class="size-3"
      />
      Stash
      <span
        v-if="git.stashList.length"
        class="font-mono text-dimmed"
      >{{ git.stashList.length }}</span>
    </button>

    <div
      v-if="open"
      class="border-t border-(--ui-border-muted) p-2 space-y-2"
    >
      <!-- Push stash -->
      <div class="flex gap-1.5">
        <UInput
          v-model="pushMessage"
          placeholder="Message (optional)"
          size="xs"
          class="flex-1 text-xs"
          @keydown.enter="doStashPush"
        />
        <UButton
          label="Stash"
          size="xs"
          color="neutral"
          variant="solid"
          :loading="pushing"
          :disabled="!git.status?.unstaged.length && !git.status?.staged.length"
          @click="doStashPush"
        />
      </div>

      <!-- Stash list -->
      <div
        v-if="git.stashList.length"
        class="space-y-1"
      >
        <div
          v-for="entry in git.stashList"
          :key="entry.index"
          class="group flex items-center gap-2 px-2 py-1.5 rounded-lg bg-muted/40 hover:bg-elevated/50 transition-colors"
        >
          <div class="flex-1 min-w-0">
            <p class="text-[11.5px] text-toned truncate">
              {{ entry.message || 'WIP' }}
            </p>
            <p class="text-[10px] font-mono text-dimmed">
              {{ entry.branch }} · {{ entry.date }}
            </p>
          </div>
          <div class="flex gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
            <UButton
              icon="i-lucide-package-open"
              size="xs"
              color="neutral"
              variant="ghost"
              aria-label="Apply stash"
              @click="doApply(entry.index)"
            />
            <UButton
              icon="i-lucide-trash-2"
              size="xs"
              color="neutral"
              variant="ghost"
              class="hover:text-red-400"
              aria-label="Drop stash"
              @click="doDrop(entry.index)"
            />
          </div>
        </div>
      </div>
      <p
        v-else
        class="text-[11px] text-dimmed italic px-1"
      >
        No stashes.
      </p>
    </div>
  </div>
</template>
