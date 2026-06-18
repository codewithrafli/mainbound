<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'

const { settings, modalOpen } = storeToRefs(useSettingsStore())
const updater = useUpdaterStore()
const toast = useToast()

const speechLanguages = [
  { label: 'Auto / mixed', value: 'auto' },
  { label: 'Indonesian', value: 'id' },
  { label: 'English', value: 'en' },
  { label: 'Japanese', value: 'ja' },
  { label: 'Korean', value: 'ko' },
  { label: 'Mandarin', value: 'zh' }
]

const speechProviders = [
  { label: 'Groq Whisper (BYOK)', value: 'groq' },
  { label: 'Browser Speech', value: 'browser' }
]

const groqKey = ref('')
const groqKeyConfigured = ref(false)
const savingGroqKey = ref(false)

async function refreshGroqStatus() {
  const status = await invoke<{ configured: boolean }>('speech_groq_key_status').catch(() => null)
  groqKeyConfigured.value = !!status?.configured
}

async function saveGroqKey() {
  if (!groqKey.value.trim()) return
  savingGroqKey.value = true
  try {
    await invoke('speech_groq_set_key', { key: groqKey.value })
    groqKey.value = ''
    await refreshGroqStatus()
    toast.add({ title: 'Groq key saved', icon: 'i-lucide-key-round' })
  } catch (error) {
    toast.add({ title: 'Failed to save Groq key', description: String(error), color: 'error' })
  } finally {
    savingGroqKey.value = false
  }
}

async function clearGroqKey() {
  await invoke('speech_groq_clear_key').catch(() => {})
  await refreshGroqStatus()
}

watch(modalOpen, (open) => {
  if (open) refreshGroqStatus()
})
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
                Provider
              </p>
              <p class="text-[11px] text-dimmed">
                Groq handles mixed Indonesian-English better
              </p>
            </div>
            <USelect
              v-model="settings.speechProvider"
              :items="speechProviders"
              size="sm"
              class="w-44"
            />
          </div>
          <div class="flex items-center justify-between gap-4">
            <div>
              <p class="text-[13px] text-toned">
                Language hint
              </p>
              <p class="text-[11px] text-dimmed">
                Auto works best for mixed speech
              </p>
            </div>
            <USelect
              v-model="settings.speechLanguage"
              :items="speechLanguages"
              size="sm"
              class="w-36"
            />
          </div>
          <div
            v-if="settings.speechProvider === 'groq'"
            class="rounded-lg border border-default bg-muted/30 p-2.5 space-y-2"
          >
            <div class="flex items-center gap-2 text-[12px]">
              <UIcon
                :name="groqKeyConfigured ? 'i-lucide-check-circle' : 'i-lucide-key-round'"
                class="size-3.5"
                :class="groqKeyConfigured ? 'text-green-500' : 'text-dimmed'"
              />
              <span :class="groqKeyConfigured ? 'text-toned' : 'text-muted'">
                {{ groqKeyConfigured ? 'Groq key configured' : 'Groq key required' }}
              </span>
            </div>
            <div class="flex gap-1.5">
              <UInput
                v-model="groqKey"
                type="password"
                placeholder="gsk_..."
                size="sm"
                class="min-w-0 flex-1 font-mono"
                @keydown.enter="saveGroqKey"
              />
              <UButton
                icon="i-lucide-save"
                color="neutral"
                variant="solid"
                size="sm"
                aria-label="Save Groq key"
                :loading="savingGroqKey"
                :disabled="!groqKey.trim()"
                @click="saveGroqKey"
              />
              <UButton
                v-if="groqKeyConfigured"
                icon="i-lucide-trash-2"
                color="error"
                variant="ghost"
                size="sm"
                aria-label="Clear Groq key"
                @click="clearGroqKey"
              />
            </div>
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
