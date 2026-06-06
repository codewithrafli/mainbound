<script setup lang="ts">
const ui = useUiStore()
const terminals = useTerminalsStore()
const workspaces = useWorkspacesStore()
const github = useGithubStore()

const connectOpen = ref(false)
const adding = ref(false)

async function addWorkspace() {
  adding.value = true
  try {
    const workspace = await workspaces.add()
    if (workspace) {
      terminals.create(workspace.path, workspace.name)
    }
  } finally {
    adding.value = false
  }
}

function skip() {
  ui.onboardingSkipped = true
  if (!terminals.tabs.length) terminals.create(null)
}

const shortcuts = [
  { kbd: '⌘K', label: 'Command palette' },
  { kbd: '⌘T', label: 'New session' },
  { kbd: '⌘D', label: 'Split pane' },
  { kbd: '⌘1 / ⌘2', label: 'Terminal · File Changes' },
  { kbd: '⌘F', label: 'Find in terminal' }
]
</script>

<template>
  <div class="absolute inset-0 z-40 flex items-center justify-center bg-default/95 backdrop-blur-sm">
    <div class="panel-card w-[420px] rounded-2xl p-6 space-y-5">
      <!-- brand -->
      <div class="flex items-center gap-3">
        <img
          src="/mainbound-logo.svg"
          alt="Mainbound"
          class="size-10 rounded-xl ring-1 ring-(--ui-border)"
          draggable="false"
        >
        <div>
          <h1 class="text-base font-semibold text-highlighted">
            Welcome to Mainbound
          </h1>
          <p class="text-[12px] text-muted">
            From shell to main.
          </p>
        </div>
      </div>

      <!-- step 1 -->
      <div class="space-y-1.5">
        <p class="section-label">
          1 · Pick your workspace
        </p>
        <p class="text-[12px] text-muted">
          A folder with your projects — Mainbound finds every git repo inside it.
        </p>
        <button
          class="btn-gradient w-full rounded-lg py-2 text-[13px] font-semibold transition disabled:opacity-60"
          :disabled="adding"
          @click="addWorkspace"
        >
          {{ adding ? 'Opening…' : 'Choose Workspace Folder' }}
        </button>
      </div>

      <!-- step 2 -->
      <div class="space-y-1.5">
        <p class="section-label">
          2 · Connect GitHub <span class="text-dimmed normal-case">(optional)</span>
        </p>
        <p class="text-[12px] text-muted">
          Pull requests, reviews, and CI — without leaving the app.
        </p>
        <UButton
          :label="github.user ? `Connected as ${github.user.login}` : 'Connect GitHub'"
          :icon="github.user ? 'i-lucide-check-circle' : 'i-simple-icons-github'"
          color="neutral"
          :variant="github.user ? 'soft' : 'outline'"
          size="sm"
          block
          :disabled="!!github.user"
          @click="connectOpen = true"
        />
      </div>

      <!-- cheatsheet -->
      <div class="rounded-lg border border-(--ui-border-muted) bg-muted/50 px-3 py-2.5 space-y-1">
        <div
          v-for="shortcut in shortcuts"
          :key="shortcut.kbd"
          class="flex items-center justify-between text-[11.5px]"
        >
          <span class="text-muted">{{ shortcut.label }}</span>
          <kbd class="font-mono text-dimmed">{{ shortcut.kbd }}</kbd>
        </div>
      </div>

      <button
        class="w-full text-center text-[11.5px] text-dimmed hover:text-muted transition-colors"
        @click="skip"
      >
        Skip — start in my home directory
      </button>
    </div>

    <GithubConnectModal v-model:open="connectOpen" />
  </div>
</template>
