<script setup lang="ts">
import type { DropdownMenuItem } from '@nuxt/ui'

const github = useGithubStore()
const cockpit = useCockpitStore()
const connectOpen = ref(false)

onMounted(() => github.init())

async function switchTo(login: string) {
  await github.switchAccount(login)
  cockpit.refresh()
}

const items = computed<DropdownMenuItem[][]>(() => [
  [{
    label: github.user?.login ?? '',
    type: 'label' as const,
    icon: 'i-simple-icons-github'
  }],
  github.accounts.map(login => ({
    label: login,
    type: 'checkbox' as const,
    checked: login === github.activeAccount,
    onSelect: () => switchTo(login)
  })),
  [
    {
      label: 'Add account…',
      icon: 'i-lucide-user-plus',
      onSelect: () => {
        connectOpen.value = true
      }
    },
    {
      label: 'Sign out',
      icon: 'i-lucide-log-out',
      onSelect: () => github.logout()
    }
  ]
])
</script>

<template>
  <UDropdownMenu
    v-if="github.user"
    :items="items"
    :content="{ side: 'right', align: 'end' }"
    :ui="{ content: 'w-52' }"
  >
    <button
      class="flex items-center justify-center rounded-full ring-1 ring-(--ui-border-accented) hover:ring-(--ui-text-dimmed) transition-shadow"
      :aria-label="github.user.login"
    >
      <UAvatar
        v-if="github.user.avatar_url"
        :src="github.user.avatar_url"
        :alt="github.user.login"
        size="xs"
      />
      <UIcon
        v-else
        name="i-simple-icons-github"
        class="size-4 m-1.5 text-toned"
      />
    </button>
  </UDropdownMenu>

  <UTooltip
    v-else
    text="Connect GitHub"
    :content="{ side: 'right' }"
  >
    <button
      class="flex items-center justify-center size-8 rounded-lg text-dimmed hover:text-toned hover:bg-elevated/50 transition-colors"
      aria-label="Connect GitHub"
      @click="connectOpen = true"
    >
      <UIcon
        name="i-simple-icons-github"
        class="size-4"
      />
    </button>
  </UTooltip>

  <GithubConnectModal v-model:open="connectOpen" />
</template>
