<script setup lang="ts">
</script>

<template>
  <aside class="flex flex-col items-center w-12 shrink-0 border-r border-default bg-muted/60">
    <!-- breathing room under the macOS traffic lights; draggable -->
    <div
      data-tauri-drag-region
      class="h-12 w-full shrink-0"
    />

    <!-- logo glyph -->
    <div class="flex items-center justify-center size-8 rounded-lg bg-elevated border border-default">
      <UIcon
        name="i-lucide-waves"
        class="size-4 text-toned"
      />
    </div>

    <div
      data-tauri-drag-region
      class="flex-1 w-full"
    />

    <!-- account -->
    <div class="pb-3">
      <GithubMenu />
    </div>
  </aside>
</template>
