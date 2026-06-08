<script setup lang="ts">
const settingsStore = useSettingsStore()
const { isMac } = usePlatform()
</script>

<template>
  <aside class="flex flex-col items-center w-12 shrink-0 border-r border-default bg-muted/60">
    <!-- breathing room under macOS traffic lights (not needed on Windows) -->
    <div
      v-if="isMac"
      data-tauri-drag-region
      class="h-12 w-full shrink-0"
    />

    <!-- brand logo -->
    <img
      src="/mainbound-logo.svg"
      alt="Mainbound"
      class="size-8 rounded-lg ring-1 ring-(--ui-border)"
      draggable="false"
    >

    <div
      data-tauri-drag-region
      class="flex-1 w-full"
    />

    <!-- settings + account -->
    <div class="flex flex-col items-center gap-2 pb-3">
      <UTooltip
        text="Settings"
        :content="{ side: 'right' }"
      >
        <button
          class="flex items-center justify-center size-8 rounded-lg text-dimmed hover:text-toned hover:bg-elevated/50 transition-colors"
          aria-label="Settings"
          @click="settingsStore.modalOpen = true"
        >
          <UIcon
            name="i-lucide-settings"
            class="size-4"
          />
        </button>
      </UTooltip>
      <GithubMenu />
    </div>
  </aside>
</template>
