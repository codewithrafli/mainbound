<script setup lang="ts">
import type { DropdownMenuItem } from '@nuxt/ui'
import { leavesOf } from '~/stores/terminals'

const terminals = useTerminalsStore()
const workspaces = useWorkspacesStore()
const notifications = useNotificationsStore()

const repoItems = computed<DropdownMenuItem[]>(() =>
  workspaces.repos.map(repo => ({
    label: repo.name,
    icon: 'i-lucide-folder-git-2',
    ...(repo.branch ? { kbds: [repo.branch] } : {}),
    onSelect: () => terminals.create(repo.path, repo.name)
  }))
)

function paneCount(tab: (typeof terminals.tabs)[number]) {
  return leavesOf(tab.root).length
}

function paneCountLabel(tab: (typeof terminals.tabs)[number]) {
  const count = paneCount(tab)
  return count === 1 ? '1 pane' : `${count} panes`
}

function onDragStart(tabId: string, event: DragEvent) {
  terminals.draggingTabId = tabId
  if (event.dataTransfer) {
    // WebKit requires data for the drag to start
    event.dataTransfer.setData('text/plain', tabId)
    event.dataTransfer.effectAllowed = 'move'
  }
}
</script>

<template>
  <aside class="flex flex-col w-56 shrink-0 border-r border-default bg-muted/40">
    <!-- SESSIONS -->
    <div class="flex items-center px-3 pt-3 pb-1.5">
      <span class="section-label">Sessions</span>
      <span
        v-if="workspaces.active"
        class="ml-1.5 text-[10px] text-dimmed truncate max-w-24"
      >· {{
        workspaces.active.name }}</span>
      <span class="ml-auto flex items-center">
        <UDropdownMenu
          v-if="repoItems.length"
          :items="repoItems"
          :content="{ align: 'end' }"
          :ui="{ content: 'w-64' }"
        >
          <UButton
            icon="i-lucide-plus"
            color="neutral"
            variant="ghost"
            size="xs"
            aria-label="New session in repository"
          />
        </UDropdownMenu>
      </span>
    </div>

    <nav class="flex-1 overflow-y-auto px-2 space-y-1 py-2">
      <div
        v-for="tab in terminals.visibleTabs"
        :key="tab.id"
        role="button"
        tabindex="0"
        draggable="true"
        class="group flex items-start w-full gap-2 px-2 py-1.5 rounded-lg text-left text-[13px] cursor-pointer transition-colors"
        :class="[
          tab.id === terminals.activeTabId
            ? 'bg-elevated text-highlighted ring-1 ring-(--ui-border-accented)'
            : 'text-muted hover:bg-elevated/50 hover:text-toned',
          tab.id === terminals.draggingTabId ? 'opacity-50' : ''
        ]"
        @click="terminals.setActiveTab(tab.id)"
        @keydown.enter="terminals.setActiveTab(tab.id)"
        @dragstart="onDragStart(tab.id, $event)"
        @dragend="terminals.draggingTabId = null"
      >
        <span class="relative shrink-0 mt-0.5">
          <!-- unread notification dot -->
          <span
            v-if="notifications.unreadByTab[tab.id]"
            class="absolute -top-1 -right-1 size-2 rounded-full bg-amber-400 ring-2 ring-(--ui-bg-muted)"
          />
        </span>
        <span class="flex min-w-0 flex-1 flex-col gap-1">
          <span class="block truncate text-[13px] leading-tight">
            {{ tab.title }}
          </span>
          <span
            v-if="tab.branch || paneCount(tab) > 1"
            class="flex min-w-0 items-center gap-1.5 text-[10.5px] leading-tight text-dimmed"
          >
            <span
              v-if="tab.branch"
              class="flex min-w-0 items-center gap-1 font-mono"
            >
              <UIcon
                name="i-lucide-git-branch"
                class="size-2.5 shrink-0"
              />
              <span class="truncate">{{ tab.branch }}</span>
            </span>
            <span
              v-if="tab.branch && paneCount(tab) > 1"
              class="size-1 rounded-full bg-current opacity-35"
            />
            <span
              v-if="paneCount(tab) > 1"
              class="inline-flex shrink-0 items-center gap-1 rounded-md bg-default/60 px-1.5 py-0.5 text-[10px] text-muted"
            >
              <UIcon
                name="i-lucide-panels-top-left"
                class="size-2.5"
              />
              {{ paneCountLabel(tab) }}
            </span>
          </span>
        </span>
        <UButton
          icon="i-lucide-x"
          color="neutral"
          variant="ghost"
          size="xs"
          class="opacity-0 group-hover:opacity-100 -mr-1"
          aria-label="Close session"
          @click.stop="terminals.killTab(tab.id)"
        />
      </div>

      <p
        v-if="!terminals.visibleTabs.length"
        class="px-2 py-4 text-xs text-dimmed italic"
      >
        No sessions yet.
      </p>
    </nav>

    <!-- workspace card, console-style -->
    <div class="p-2">
      <div class="panel-card p-3 space-y-2">
        <p class="text-[10.5px] text-dimmed">
          Workspace
        </p>
        <p class="text-[13px] font-medium text-highlighted truncate">
          {{ workspaces.active?.name ?? 'None selected' }}
        </p>
        <p
          v-if="workspaces.repos.length"
          class="text-[10.5px] font-mono text-muted"
        >
          {{ workspaces.repos.length }} repositories
        </p>
        <UButton
          label="Add Workspace"
          icon="i-lucide-folder-plus"
          color="neutral"
          variant="outline"
          size="xs"
          block
          @click="workspaces.add()"
        />
      </div>
    </div>
  </aside>
</template>
