<script setup lang="ts">
import type { DropdownMenuItem } from '@nuxt/ui'

const terminals = useTerminalsStore()
const workspaces = useWorkspacesStore()

function newSession() {
  terminals.create(workspaces.active?.path ?? null, workspaces.active?.name)
}

const repoItems = computed<DropdownMenuItem[]>(() =>
  workspaces.repos.map(repo => ({
    label: repo.name,
    icon: 'i-lucide-folder-git-2',
    ...(repo.branch ? { kbds: [repo.branch] } : {}),
    onSelect: () => terminals.create(repo.path, repo.name)
  }))
)
</script>

<template>
  <aside class="flex flex-col w-60 shrink-0 border-r border-default bg-muted">
    <div class="flex gap-1 p-2">
      <UButton
        label="New Session"
        icon="i-lucide-plus"
        color="neutral"
        variant="outline"
        size="sm"
        block
        class="rounded-full flex-1"
        @click="newSession"
      />
      <UDropdownMenu
        v-if="repoItems.length"
        :items="repoItems"
        :content="{ align: 'end' }"
        :ui="{ content: 'w-64' }"
      >
        <UButton
          icon="i-lucide-chevron-down"
          color="neutral"
          variant="outline"
          size="sm"
          class="rounded-full"
          aria-label="New session in repository"
        />
      </UDropdownMenu>
    </div>

    <div class="px-3 pt-2 pb-1 text-[10px] font-medium tracking-wider text-dimmed uppercase">
      Sessions
    </div>

    <nav class="flex-1 overflow-y-auto px-2 space-y-0.5">
      <div
        v-for="session in terminals.sessions"
        :key="session.id"
        role="button"
        tabindex="0"
        class="group flex items-center w-full gap-2 px-2 py-1.5 rounded-md text-left text-sm cursor-pointer transition-colors"
        :class="session.id === terminals.activeId
          ? 'bg-elevated text-highlighted'
          : 'text-muted hover:bg-elevated/50 hover:text-toned'"
        @click="terminals.setActive(session.id)"
        @keydown.enter="terminals.setActive(session.id)"
      >
        <UIcon name="i-lucide-terminal" class="size-3.5 shrink-0" />
        <span class="flex-1 min-w-0">
          <span class="block truncate leading-tight">{{ session.title }}</span>
          <span
            v-if="session.branch"
            class="flex items-center gap-1 text-[11px] text-dimmed leading-tight"
          >
            <UIcon name="i-lucide-git-branch" class="size-3" />
            <span class="truncate">{{ session.branch }}</span>
          </span>
        </span>
        <UButton
          icon="i-lucide-x"
          color="neutral"
          variant="ghost"
          size="xs"
          class="opacity-0 group-hover:opacity-100 -mr-1"
          aria-label="Close session"
          @click.stop="terminals.kill(session.id)"
        />
      </div>

      <p v-if="!terminals.sessions.length" class="px-2 py-4 text-xs text-dimmed italic">
        No sessions yet.
      </p>
    </nav>
  </aside>
</template>
