<script setup lang="ts">
const terminals = useTerminalsStore()

// Open a first session on launch
onMounted(() => {
  if (!terminals.sessions.length) terminals.create()
})

function onExited(id: string) {
  terminals.remove(id)
}
</script>

<template>
  <div class="flex h-full min-h-0">
    <TerminalSessionSidebar />

    <div class="relative flex-1 min-w-0 bg-[#0d0d0d]">
      <!-- Panes stay mounted (v-show) so scrollback survives switching -->
      <TerminalTerminalPane
        v-for="session in terminals.sessions"
        v-show="session.id === terminals.activeId"
        :key="session.id"
        :session-id="session.id"
        :cwd="session.cwd"
        class="absolute inset-0"
        @exited="onExited(session.id)"
      />

      <div
        v-if="!terminals.sessions.length"
        class="absolute inset-0 flex items-center justify-center"
      >
        <div class="text-center space-y-2">
          <UIcon name="i-lucide-terminal" class="size-8 text-dimmed" />
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
            @click="terminals.create()"
          />
        </div>
      </div>
    </div>
  </div>
</template>
