<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'

const open = defineModel<boolean>('open', { default: false })

const github = useGithubStore()

const pat = ref('')
const busy = ref(false)
const error = ref<string | null>(null)

// --- device flow state ---
const showDevice = ref(false)
const clientId = ref('')
const userCode = ref<string | null>(null)
const verificationUri = ref('')
let polling = false

async function connectWithPat() {
  if (!pat.value.trim()) return
  busy.value = true
  error.value = null
  try {
    await github.connectPat(pat.value.trim())
    pat.value = ''
    open.value = false
  } catch (e) {
    error.value = String(e)
  } finally {
    busy.value = false
  }
}

interface DeviceCode {
  device_code: string
  user_code: string
  verification_uri: string
  interval: number
  expires_in: number
}

async function startDeviceFlow() {
  if (!clientId.value.trim()) return
  busy.value = true
  error.value = null
  try {
    const code = await invoke<DeviceCode>('gh_device_start', { clientId: clientId.value.trim() })
    userCode.value = code.user_code
    verificationUri.value = code.verification_uri
    const { openUrl } = await import('@tauri-apps/plugin-opener')
    openUrl(code.verification_uri).catch(() => {})
    pollDevice(code)
  } catch (e) {
    error.value = String(e)
  } finally {
    busy.value = false
  }
}

async function pollDevice(code: DeviceCode) {
  polling = true
  let interval = Math.max(code.interval, 5) * 1000
  const deadline = Date.now() + code.expires_in * 1000

  while (polling && open.value && Date.now() < deadline) {
    await new Promise(resolve => setTimeout(resolve, interval))
    if (!polling || !open.value) return
    try {
      const result = await invoke<{ status: string, user: typeof github.user }>('gh_device_poll', {
        clientId: clientId.value.trim(),
        deviceCode: code.device_code
      })
      if (result.status === 'ok') {
        github.user = result.user
        userCode.value = null
        open.value = false
        return
      }
      if (result.status === 'slow_down') interval += 5000
      if (result.status === 'expired' || result.status === 'denied') {
        error.value = result.status === 'denied' ? 'Authorization was denied.' : 'Code expired — try again.'
        userCode.value = null
        return
      }
    } catch (e) {
      error.value = String(e)
      userCode.value = null
      return
    }
  }
}

watch(open, (isOpen) => {
  if (!isOpen) {
    polling = false
    userCode.value = null
    error.value = null
  }
})
</script>

<template>
  <UModal
    v-model:open="open"
    title="Connect GitHub"
    :ui="{ content: 'max-w-md' }"
  >
    <template #body>
      <div class="space-y-4">
        <!-- PAT -->
        <div class="space-y-2">
          <p class="text-xs text-muted">
            Paste a personal access token with <code class="text-toned">repo</code> scope.
            Stored securely in the macOS Keychain.
          </p>
          <div class="flex gap-2">
            <UInput
              v-model="pat"
              type="password"
              placeholder="ghp_… or github_pat_…"
              size="sm"
              class="flex-1"
              @keydown.enter="connectWithPat"
            />
            <UButton
              label="Connect"
              color="primary"
              size="sm"
              :loading="busy && !showDevice"
              :disabled="!pat.trim()"
              @click="connectWithPat"
            />
          </div>
        </div>

        <USeparator label="or" />

        <!-- Device flow -->
        <UCollapsible v-model:open="showDevice">
          <UButton
            label="Use OAuth device flow"
            icon="i-lucide-monitor-smartphone"
            color="neutral"
            variant="link"
            size="xs"
            trailing-icon="i-lucide-chevron-down"
          />
          <template #content>
            <div class="space-y-2 pt-2">
              <p class="text-xs text-muted">
                Requires a GitHub OAuth App client ID (Settings → Developer settings → OAuth Apps, enable Device Flow).
              </p>
              <template v-if="!userCode">
                <div class="flex gap-2">
                  <UInput
                    v-model="clientId"
                    placeholder="OAuth App client ID"
                    size="sm"
                    class="flex-1"
                  />
                  <UButton
                    label="Start"
                    color="neutral"
                    variant="outline"
                    size="sm"
                    :loading="busy && showDevice"
                    :disabled="!clientId.trim()"
                    @click="startDeviceFlow"
                  />
                </div>
              </template>
              <template v-else>
                <div class="rounded-lg border border-default bg-elevated p-3 text-center space-y-1">
                  <p class="text-xs text-muted">
                    Enter this code at <span class="text-toned">{{ verificationUri }}</span>
                  </p>
                  <p class="text-xl font-mono font-semibold tracking-widest text-highlighted">
                    {{ userCode }}
                  </p>
                  <p class="text-[10px] text-dimmed">
                    Waiting for authorization…
                  </p>
                </div>
              </template>
            </div>
          </template>
        </UCollapsible>

        <UAlert
          v-if="error"
          color="error"
          variant="soft"
          :description="error"
        />
      </div>
    </template>
  </UModal>
</template>
