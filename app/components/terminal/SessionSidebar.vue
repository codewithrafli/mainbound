<script setup lang="ts">
const terminals = useTerminalsStore()

function shortCwd(cwd: string | null) {
  if (!cwd) return '~'
  return cwd.split('/').filter(Boolean).pop() ?? '~'
}
</script>

<template>
  <aside class="flex flex-col w-56 shrink-0 border-r border-default bg-muted">
    <div class="p-2">
      <UButton
        label="New Session"
        icon="i-lucide-plus"
        color="neutral"
        variant="outline"
        size="sm"
        block
        class="rounded-full"
        @click="terminals.create()"
      />
    </div>

    <div class="px-3 pt-2 pb-1 text-[10px] font-medium tracking-wider text-dimmed uppercase">
      Sessions
    </div>

    <nav class="flex-1 overflow-y-auto px-2 space-y-0.5">
      <button
        v-for="session in terminals.sessions"
        :key="session.id"
        class="group flex items-center w-full gap-2 px-2 py-1.5 rounded-md text-left text-sm transition-colors"
        :class="session.id === terminals.activeId
          ? 'bg-elevated text-highlighted'
          : 'text-muted hover:bg-elevated/50 hover:text-toned'"
        @click="terminals.setActive(session.id)"
      >
        <UIcon name="i-lucide-terminal" class="size-3.5 shrink-0" />
        <span class="flex-1 truncate">{{ session.title }} · {{ shortCwd(session.cwd) }}</span>
        <UButton
          icon="i-lucide-x"
          color="neutral"
          variant="ghost"
          size="xs"
          class="opacity-0 group-hover:opacity-100 -mr-1"
          aria-label="Close session"
          @click.stop="terminals.kill(session.id)"
        />
      </button>

      <p v-if="!terminals.sessions.length" class="px-2 py-4 text-xs text-dimmed italic">
        No sessions yet.
      </p>
    </nav>
  </aside>
</template>
