<script setup lang="ts">
const terminals = useTerminalsStore()
</script>

<template>
  <footer class="flex items-center h-6 shrink-0 px-3 gap-4 border-t border-default bg-muted text-[11px] text-muted select-none">
    <span class="flex items-center gap-1">
      <UIcon name="i-lucide-terminal" class="size-3" />
      {{ terminals.sessions.length }} session{{ terminals.sessions.length === 1 ? '' : 's' }}
    </span>

    <span class="ml-auto flex items-center gap-1">
      <span class="size-1.5 rounded-full bg-green-500" />
      Ready
    </span>
  </footer>
</template>
