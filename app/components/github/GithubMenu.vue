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
      label: `Sign out ${github.activeAccount ?? ''}`,
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
    :content="{ align: 'end' }"
    :ui="{ content: 'w-52' }"
  >
    <UButton
      color="neutral"
      variant="ghost"
      size="xs"
      class="gap-1.5"
    >
      <UAvatar
        v-if="github.user.avatar_url"
        :src="github.user.avatar_url"
        :alt="github.user.login"
        size="3xs"
      />
      <UIcon
        v-else
        name="i-simple-icons-github"
        class="size-3.5"
      />
      <span class="text-xs">{{ github.user.login }}</span>
      <UIcon
        name="i-lucide-chevron-down"
        class="size-3 text-dimmed"
      />
    </UButton>
  </UDropdownMenu>

  <UButton
    v-else
    label="Connect GitHub"
    icon="i-simple-icons-github"
    color="neutral"
    variant="outline"
    size="xs"
    class="rounded-full"
    @click="connectOpen = true"
  />

  <GithubConnectModal v-model:open="connectOpen" />
</template>
