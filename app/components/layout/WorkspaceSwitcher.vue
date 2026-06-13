<script setup lang="ts">
import type { DropdownMenuItem } from '@nuxt/ui'
import type { Workspace } from '~/stores/workspaces'

const workspaces = useWorkspacesStore()
const terminals = useTerminalsStore()
const managerOpen = ref(false)
const deleting = ref<Workspace | null>(null)
const removing = ref(false)

const items = computed<DropdownMenuItem[][]>(() => [
  workspaces.list.map(w => ({
    label: w.name,
    type: 'checkbox' as const,
    checked: w.id === workspaces.activeId,
    onSelect: () => workspaces.setActive(w.id)
  })),
  [{
    label: 'Add Workspace…',
    icon: 'i-lucide-folder-plus',
    onSelect: () => workspaces.add()
  }, {
    label: 'Manage Workspaces…',
    icon: 'i-lucide-list-ordered',
    disabled: workspaces.list.length === 0,
    onSelect: () => { managerOpen.value = true }
  }]
])

async function removeWorkspace() {
  const workspace = deleting.value
  if (!workspace || removing.value) return
  removing.value = true
  try {
    await workspaces.remove(workspace.id)
    await terminals.killWorkspace(workspace.id)
    deleting.value = null
    if (!workspaces.list.length) managerOpen.value = false
  } finally {
    removing.value = false
  }
}
</script>

<template>
  <div>
    <UDropdownMenu
      :items="items"
      :content="{ align: 'start' }"
      :ui="{ content: 'w-56' }"
    >
      <!-- plain text + chevron, like the reference's context switcher -->
      <button class="flex items-center gap-1.5 px-2 py-1 rounded-lg text-sm font-medium text-highlighted hover:bg-elevated/60 transition-colors">
        <UIcon
          name="i-lucide-folder"
          class="size-3.5 text-muted"
        />
        {{ workspaces.active?.name ?? 'No workspace' }}
        <UIcon
          name="i-lucide-chevron-down"
          class="size-3 text-dimmed"
        />
      </button>
    </UDropdownMenu>

    <UModal
      v-model:open="managerOpen"
      title="Manage Workspaces"
      :ui="{ content: 'max-w-xl' }"
    >
      <template #body>
        <div class="space-y-3">
          <div
            v-for="(workspace, index) in workspaces.list"
            :key="workspace.id"
            class="flex items-center gap-2 rounded-lg border border-default bg-muted/30 px-3 py-2"
          >
            <div class="min-w-0 flex-1">
              <div class="flex items-center gap-2">
                <p class="truncate text-sm font-medium text-toned">
                  {{ workspace.name }}
                </p>
                <UBadge
                  v-if="workspace.id === workspaces.activeId"
                  label="Active"
                  color="neutral"
                  variant="soft"
                  size="sm"
                />
              </div>
              <p class="truncate font-mono text-[11px] text-dimmed">
                {{ workspace.path }}
              </p>
            </div>

            <div class="flex items-center gap-1">
              <UTooltip text="Move up">
                <UButton
                  icon="i-lucide-arrow-up"
                  color="neutral"
                  variant="ghost"
                  size="xs"
                  :disabled="index === 0"
                  aria-label="Move workspace up"
                  @click="workspaces.move(workspace.id, -1)"
                />
              </UTooltip>
              <UTooltip text="Move down">
                <UButton
                  icon="i-lucide-arrow-down"
                  color="neutral"
                  variant="ghost"
                  size="xs"
                  :disabled="index === workspaces.list.length - 1"
                  aria-label="Move workspace down"
                  @click="workspaces.move(workspace.id, 1)"
                />
              </UTooltip>
              <UTooltip text="Remove workspace">
                <UButton
                  icon="i-lucide-trash-2"
                  color="error"
                  variant="ghost"
                  size="xs"
                  aria-label="Remove workspace"
                  @click="deleting = workspace"
                />
              </UTooltip>
            </div>
          </div>

          <UButton
            label="Add Workspace"
            icon="i-lucide-folder-plus"
            color="neutral"
            variant="outline"
            size="sm"
            @click="workspaces.add()"
          />
        </div>
      </template>
    </UModal>

    <UModal
      :open="!!deleting"
      title="Remove Workspace?"
      :ui="{ content: 'max-w-md' }"
      @update:open="(open) => { if (!open) deleting = null }"
    >
      <template #body>
        <div class="space-y-4">
          <p class="text-sm text-muted">
            Remove <span class="font-medium text-toned">{{ deleting?.name }}</span> from Mainbound?
            Its terminal sessions will be closed. Files on disk are not deleted.
          </p>
          <div class="flex justify-end gap-2">
            <UButton
              label="Cancel"
              color="neutral"
              variant="ghost"
              size="sm"
              :disabled="removing"
              @click="deleting = null"
            />
            <UButton
              label="Remove"
              icon="i-lucide-trash-2"
              color="error"
              variant="solid"
              size="sm"
              :loading="removing"
              @click="removeWorkspace"
            />
          </div>
        </div>
      </template>
    </UModal>
  </div>
</template>
