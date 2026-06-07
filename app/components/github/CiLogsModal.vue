<script setup lang="ts">
const github = useGithubStore()
const termEl = ref<HTMLPreElement>()

watch(github.ciLogsContent, () => {
  nextTick(() => {
    if (termEl.value) termEl.value.scrollTop = termEl.value.scrollHeight
  })
})
</script>

<template>
  <UModal
    v-model:open="github.ciLogsModalOpen"
    :title="github.ciLogsJobName || 'CI Logs'"
    :ui="{ content: 'max-w-4xl h-[70vh] flex flex-col' }"
  >
    <template #body>
      <div class="flex flex-col flex-1 min-h-0 gap-3">
        <!-- Job list -->
        <div
          v-if="github.workflowJobs.length"
          class="flex gap-1.5 flex-wrap shrink-0"
        >
          <span
            v-for="job in github.workflowJobs"
            :key="job.id"
            class="flex items-center gap-1 text-[10.5px] font-mono px-2 py-1 rounded-full border"
            :class="{
              'border-green-500/30 text-green-400': job.conclusion === 'success',
              'border-red-500/30 text-red-400': job.conclusion === 'failure',
              'border-amber-500/30 text-amber-400': job.status === 'in_progress',
              'border-(--ui-border) text-dimmed': !job.conclusion && job.status !== 'in_progress',
            }"
          >
            <span
              class="size-1.5 rounded-full"
              :class="{
                'bg-green-500': job.conclusion === 'success',
                'bg-red-500': job.conclusion === 'failure',
                'bg-amber-400 animate-pulse': job.status === 'in_progress',
                'bg-dimmed': !job.conclusion && job.status !== 'in_progress',
              }"
            />
            {{ job.name }}
          </span>
        </div>

        <!-- Log output -->
        <pre
          ref="termEl"
          class="flex-1 min-h-0 overflow-auto rounded-lg bg-(--ui-bg) border border-(--ui-border) p-3 text-[11.5px] font-mono leading-relaxed text-toned whitespace-pre-wrap"
        >
          <span
            v-if="github.ciLogsLoading"
            class="text-dimmed"
          >Loading logs…</span>
          <span v-else>{{ github.ciLogsContent || 'No log output.' }}</span>
        </pre>
      </div>
    </template>
  </UModal>
</template>
