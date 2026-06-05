<script setup lang="ts">
const ui = useUiStore()
</script>

<template>
  <div class="h-screen flex flex-col overflow-hidden bg-default text-default">
    <LayoutTopBar />

    <!-- v-show (not v-if): terminal panes must stay mounted across view
         toggles so PTYs and xterm scrollback survive -->
    <main class="flex-1 min-h-0">
      <TerminalTerminalView v-show="ui.view === 'terminal'" class="h-full" />
      <GitFileChangesView v-show="ui.view === 'changes'" class="h-full" />
    </main>

    <LayoutStatusBar />
  </div>
</template>
