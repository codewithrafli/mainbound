<script setup lang="ts">
const { settings, modalOpen } = storeToRefs(useSettingsStore())
const updater = useUpdaterStore()
</script>

<template>
  <UModal
    v-model:open="modalOpen"
    title="Settings"
    :ui="{ content: 'max-w-md' }"
  >
    <template #body>
      <div class="space-y-4">
        <!-- terminal -->
        <div class="space-y-3">
          <p class="section-label">
            Terminal
          </p>
          <div class="flex items-center justify-between gap-4">
            <div>
              <p class="text-[13px] text-toned">
                Font size
              </p>
              <p class="text-[11px] text-dimmed">
                Applies to all terminals instantly
              </p>
            </div>
            <UInputNumber
              v-model="settings.fontSize"
              :min="9"
              :max="24"
              :step="0.5"
              size="sm"
              class="w-28"
            />
          </div>
          <div class="flex items-center justify-between gap-4">
            <div>
              <p class="text-[13px] text-toned">
                Shell
              </p>
              <p class="text-[11px] text-dimmed">
                New sessions only · empty = $SHELL
              </p>
            </div>
            <UInput
              v-model="settings.shell"
              placeholder="/bin/zsh"
              size="sm"
              class="w-40 font-mono"
            />
          </div>
        </div>

        <USeparator />

        <!-- AI -->
        <div class="space-y-3">
          <p class="section-label">
            AI
          </p>
          <div class="flex items-center justify-between gap-4">
            <div>
              <p class="text-[13px] text-toned">
                Provider
              </p>
              <p class="text-[11px] text-dimmed">
                Used for “Generate with AI” in commits and PRs
              </p>
            </div>
            <USelect
              v-model="settings.aiProvider"
              :items="[
                { label: 'Claude Code', value: 'claude' },
                { label: 'Codex', value: 'codex' }
              ]"
              size="sm"
              class="w-36"
            />
          </div>
        </div>

        <USeparator />

        <!-- notifications -->
        <div class="space-y-3">
          <p class="section-label">
            Notifications
          </p>
          <div class="flex items-center justify-between gap-4">
            <div>
              <p class="text-[13px] text-toned">
                "Command finished" threshold
              </p>
              <p class="text-[11px] text-dimmed">
                Minimum seconds of output before notifying
              </p>
            </div>
            <UInputNumber
              v-model="settings.notifMinBurst"
              :min="3"
              :max="120"
              size="sm"
              class="w-28"
            />
          </div>
        </div>

        <USeparator />

        <!-- speech -->
        <div class="space-y-3">
          <p class="section-label">
            Speech
          </p>
          <div class="flex items-center justify-between gap-4">
            <div>
              <p class="text-[13px] text-toned">
                Dictation language
              </p>
              <p class="text-[11px] text-dimmed">
                Used by terminal speech-to-text · empty = system default
              </p>
            </div>
            <UInput
              v-model="settings.speechLanguage"
              placeholder="en-US"
              size="sm"
              class="w-28 font-mono"
            />
          </div>
        </div>

        <USeparator />

        <!-- updates -->
        <div class="space-y-3">
          <p class="section-label">
            Updates
          </p>
          <div class="flex items-center justify-between gap-4">
            <div>
              <p class="text-[13px] text-toned">
                Check automatically on launch
              </p>
              <p class="text-[11px] text-dimmed">
                Current: v{{ updater.currentVersion || '…' }}
              </p>
            </div>
            <USwitch v-model="settings.autoUpdateCheck" />
          </div>

          <!-- Auto draft PR -->
          <div class="flex items-center justify-between gap-4">
            <div>
              <p class="text-[12.5px] text-toned font-medium">
                Auto Draft PR on Push
              </p>
              <p class="text-[11px] text-dimmed">
                Automatically create a draft PR with AI description when pushing a new branch.
              </p>
            </div>
            <USwitch v-model="settings.autoDraftPr" />
          </div>

          <UButton
            label="Check for Updates now"
            icon="i-lucide-arrow-up-circle"
            color="neutral"
            variant="outline"
            size="xs"
            :loading="updater.checking"
            @click="updater.check(true)"
          />
        </div>
      </div>
    </template>
  </UModal>
</template>
