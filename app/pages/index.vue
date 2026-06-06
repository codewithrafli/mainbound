<script setup lang="ts">
const ui = useUiStore()
const workspaces = useWorkspacesStore()
const terminals = useTerminalsStore()

const showOnboarding = computed(() =>
  workspaces.initialized
  && !workspaces.list.length
  && !terminals.tabs.length
  && !ui.onboardingSkipped
)
</script>

<template>
  <!-- floating frame on a pure-black canvas, console-style -->
  <div class="h-screen w-screen bg-black p-2">
    <div class="flex h-full w-full overflow-hidden rounded-2xl border border-(--ui-border) bg-default text-default shadow-[0_0_40px_rgba(0,0,0,0.8)]">
      <LayoutIconRail />

      <div class="flex flex-col flex-1 min-w-0">
        <LayoutTopBar />

        <!-- v-show (not v-if): terminal panes must stay mounted across view
             toggles so PTYs and xterm scrollback survive -->
        <main class="relative flex-1 min-h-0">
          <TerminalView
            v-show="ui.view === 'terminal'"
            class="h-full"
          />
          <GitFileChangesView
            v-show="ui.view === 'changes'"
            class="h-full"
          />
          <OnboardingOverlay v-if="showOnboarding" />
        </main>

        <LayoutStatusBar />
      </div>
    </div>
  </div>
</template>
