<script setup lang="ts">
const updater = useUpdaterStore()
</script>

<template>
  <UModal
    v-model:open="updater.modalOpen"
    :title="updater.available ? `Update available — v${updater.newVersion}` : 'Check for Updates'"
    :ui="{ content: 'max-w-lg' }"
  >
    <template #body>
      <div class="space-y-3">
        <template v-if="updater.available">
          <p class="text-xs text-muted">
            You're on <span class="font-mono text-toned">v{{ updater.currentVersion }}</span> —
            <span class="font-mono text-highlighted">v{{ updater.newVersion }}</span> is ready
            <template v-if="updater.releaseDate">
              ({{ updater.releaseDate.slice(0, 10) }})
            </template>.
          </p>

          <!-- changelog -->
          <div
            v-if="updater.changelog"
            class="panel-card max-h-72 overflow-y-auto p-3"
          >
            <MarkdownBody :source="updater.changelog" />
          </div>

          <div
            v-if="updater.installing"
            class="space-y-1.5"
          >
            <UProgress
              :model-value="updater.progress"
              size="sm"
            />
            <p class="text-[11px] text-dimmed text-center">
              Downloading… {{ updater.progress }}%
            </p>
          </div>

          <UButton
            v-else
            label="Download & Restart"
            icon="i-lucide-download"
            color="neutral"
            variant="solid"
            size="sm"
            block
            @click="updater.installAndRelaunch()"
          />
        </template>

        <template v-else-if="updater.upToDate">
          <div class="flex items-center gap-2 text-sm text-toned">
            <UIcon
              name="i-lucide-check-circle"
              class="size-4 text-green-500"
            />
            You're up to date — <span class="font-mono">v{{ updater.currentVersion }}</span>
          </div>
        </template>

        <UAlert
          v-if="updater.error"
          color="error"
          variant="soft"
          :description="updater.error"
          class="text-xs"
        />
      </div>
    </template>
  </UModal>
</template>
