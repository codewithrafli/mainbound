<script setup lang="ts">
const git = useGitStore()
const toast = useToast()

const open = ref(false)
const name = ref('')
const saving = ref(false)

async function save() {
  if (!git.selectedRepo) return
  saving.value = true
  const ok = await git.checkpointSave(git.selectedRepo, name.value.trim() || undefined)
  saving.value = false
  if (ok) {
    name.value = ''
    toast.add({ title: 'Checkpoint saved', icon: 'i-lucide-flag' })
  }
}

async function restore(index: number) {
  if (!git.selectedRepo) return
  const ok = await git.checkpointRestore(git.selectedRepo, index)
  if (ok) toast.add({ title: 'Checkpoint restored', icon: 'i-lucide-rotate-ccw' })
}

async function drop(index: number) {
  if (!git.selectedRepo) return
  await git.checkpointDrop(git.selectedRepo, index)
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
        name="i-lucide-flag"
        class="size-3"
      />
      Checkpoints
      <span
        v-if="git.checkpoints.length"
        class="font-mono text-dimmed"
      >
        {{ git.checkpoints.length }}
      </span>
      <UTooltip
        text="Save a named snapshot before letting AI make big changes"
        :content="{ side: 'top' }"
      >
        <UIcon
          name="i-lucide-info"
          class="size-3 text-dimmed ml-auto"
        />
      </UTooltip>
    </button>

    <div
      v-if="open"
      class="border-t border-(--ui-border-muted) p-2 space-y-2"
    >
      <!-- Save new checkpoint -->
      <div class="flex gap-1.5">
        <UInput
          v-model="name"
          placeholder="Name (e.g. before AI refactor)"
          size="xs"
          class="flex-1 text-xs"
          @keydown.enter="save"
        />
        <UButton
          label="Save"
          size="xs"
          color="neutral"
          variant="solid"
          :loading="saving"
          @click="save"
        />
      </div>

      <!-- Checkpoint list -->
      <div
        v-if="git.checkpoints.length"
        class="space-y-1"
      >
        <div
          v-for="cp in git.checkpoints"
          :key="cp.index"
          class="group flex items-center gap-2 px-2 py-1.5 rounded-lg bg-purple-500/5 border border-purple-500/20 hover:bg-purple-500/10 transition-colors"
        >
          <UIcon
            name="i-lucide-flag"
            class="size-3 text-purple-400 shrink-0"
          />
          <div class="flex-1 min-w-0">
            <p class="text-[11.5px] text-toned truncate">
              {{ cp.message.replace('checkpoint: ', '') }}
            </p>
            <p class="text-[10px] font-mono text-dimmed">
              {{ cp.branch }} · {{ cp.date }}
            </p>
          </div>
          <div class="flex gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
            <UButton
              icon="i-lucide-rotate-ccw"
              size="xs"
              color="neutral"
              variant="ghost"
              aria-label="Restore checkpoint"
              @click="restore(cp.index)"
            />
            <UButton
              icon="i-lucide-trash-2"
              size="xs"
              color="neutral"
              variant="ghost"
              class="hover:text-red-400"
              aria-label="Delete checkpoint"
              @click="drop(cp.index)"
            />
          </div>
        </div>
      </div>
      <p
        v-else
        class="text-[11px] text-dimmed italic px-1"
      >
        No checkpoints yet.
      </p>
    </div>
  </div>
</template>
