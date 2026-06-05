<script setup lang="ts">
const terminals = useTerminalsStore()
const workspaces = useWorkspacesStore()

// Restore workspaces, then open a first session in the active one
onMounted(async () => {
  await workspaces.init()
  if (!terminals.tabs.length) {
    terminals.create(workspaces.active?.path ?? null, workspaces.active?.name)
  }
})
</script>

<template>
  <div class="flex h-full min-h-0">
    <TerminalSessionSidebar />

    <div class="relative flex-1 min-w-0 bg-[#0d0d0d]">
      <!-- All tabs stay mounted (v-show) so PTYs + scrollback survive switching -->
      <TerminalTabView
        v-for="tab in terminals.tabs"
        v-show="tab.id === terminals.activeTabId"
        :key="tab.id"
        :tab="tab"
        class="absolute inset-0"
      />

      <div
        v-if="!terminals.tabs.length"
        class="absolute inset-0 flex items-center justify-center"
      >
        <div class="text-center space-y-2">
          <UIcon
            name="i-lucide-terminal"
            class="size-8 text-dimmed"
          />
          <p class="text-sm text-muted">
            No open sessions
          </p>
          <UButton
            label="New Session"
            icon="i-lucide-plus"
            color="neutral"
            variant="outline"
            size="sm"
            class="rounded-full"
            @click="terminals.create(workspaces.active?.path ?? null, workspaces.active?.name)"
          />
        </div>
      </div>
    </div>
  </div>
</template>
