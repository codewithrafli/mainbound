<script setup lang="ts">
// Placeholder until M3 (git status / diff / commit)
</script>

<template>
  <div class="flex h-full items-center justify-center">
    <div class="flex flex-col items-center gap-3 px-10 py-12 rounded-xl border border-default bg-muted">
      <UIcon name="i-lucide-git-branch" class="size-8 text-dimmed" />
      <h2 class="text-base font-medium text-highlighted">
        File Changes
      </h2>
      <p class="text-sm text-muted">
        Pick a file from the left sidebar to see its diff here.
      </p>
    </div>
  </div>
</template>
