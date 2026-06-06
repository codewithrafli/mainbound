<script setup lang="ts">
const ui = useUiStore()
const terminals = useTerminalsStore()
const workspaces = useWorkspacesStore()
const git = useGitStore()
const github = useGithubStore()
const cockpit = useCockpitStore()
const updater = useUpdaterStore()

function run(action: () => void) {
  return () => {
    ui.paletteOpen = false
    action()
  }
}

const activeRepo = computed(() => cockpit.activeRepo ?? git.selectedRepo)

const groups = computed(() => [
  {
    id: 'actions',
    label: 'Actions',
    items: [
      {
        label: 'New Session',
        icon: 'i-lucide-plus',
        kbds: ['meta', 'T'],
        onSelect: run(() => {
          ui.view = 'terminal'
          terminals.create(workspaces.active?.path ?? null, workspaces.active?.name)
        })
      },
      {
        label: 'Split Right',
        icon: 'i-lucide-columns-2',
        kbds: ['meta', 'D'],
        onSelect: run(() => terminals.split('row'))
      },
      {
        label: 'Split Down',
        icon: 'i-lucide-rows-2',
        kbds: ['shift', 'meta', 'D'],
        onSelect: run(() => terminals.split('column'))
      },
      {
        label: 'Terminal view',
        icon: 'i-lucide-terminal',
        kbds: ['meta', '1'],
        onSelect: run(() => {
          ui.view = 'terminal'
        })
      },
      {
        label: 'File Changes view',
        icon: 'i-lucide-git-branch',
        kbds: ['meta', '2'],
        onSelect: run(() => {
          ui.view = 'changes'
        })
      },
      ...(activeRepo.value
        ? [
            {
              label: 'Stage all changes',
              icon: 'i-lucide-layers',
              onSelect: run(() => git.stageAll(activeRepo.value!))
            },
            {
              label: 'Push',
              icon: 'i-lucide-arrow-up-from-line',
              onSelect: run(async () => {
                if (await github.push(activeRepo.value!)) cockpit.refresh()
              })
            },
            {
              label: 'Pull',
              icon: 'i-lucide-arrow-down-to-line',
              onSelect: run(async () => {
                if (await github.pull(activeRepo.value!)) cockpit.refresh()
              })
            }
          ]
        : []),
      {
        label: 'Add Workspace…',
        icon: 'i-lucide-folder-plus',
        onSelect: run(() => workspaces.add())
      },
      {
        label: 'Check for Updates…',
        icon: 'i-lucide-arrow-up-circle',
        onSelect: run(() => updater.check(true))
      }
    ]
  },
  {
    id: 'sessions',
    label: 'Sessions',
    items: terminals.tabs.map(tab => ({
      label: tab.title,
      icon: 'i-lucide-terminal',
      suffix: tab.branch ?? undefined,
      onSelect: run(() => {
        terminals.setActiveTab(tab.id)
        ui.view = 'terminal'
      })
    }))
  },
  {
    id: 'repositories',
    label: 'Repositories',
    items: workspaces.repos.map(repo => ({
      label: repo.name,
      icon: 'i-lucide-folder-git-2',
      suffix: repo.branch ?? undefined,
      onSelect: run(() => {
        git.selectRepo(repo.path)
        ui.view = 'changes'
      })
    }))
  }
])
</script>

<template>
  <UModal
    v-model:open="ui.paletteOpen"
    :ui="{ content: 'max-w-xl' }"
  >
    <template #content>
      <UCommandPalette
        :groups="groups"
        placeholder="Type a command, session, or repository…"
        class="h-96"
        :close="false"
      />
    </template>
  </UModal>
</template>
