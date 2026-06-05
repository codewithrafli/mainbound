<script setup lang="ts">
const props = defineProps<{ status: string }>()

const styles: Record<string, string> = {
  M: 'text-amber-400',
  A: 'text-green-400',
  U: 'text-green-400',
  D: 'text-red-400',
  R: 'text-blue-400',
  C: 'text-blue-400',
  '!': 'text-orange-500'
}

const cls = computed(() => styles[props.status] ?? 'text-muted')
</script>

<template>
  <span class="w-3.5 shrink-0 text-center font-mono text-xs font-semibold" :class="cls">
    {{ status }}
  </span>
</template>
