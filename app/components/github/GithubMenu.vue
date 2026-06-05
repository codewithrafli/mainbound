<script setup lang="ts">
import type { DropdownMenuItem } from '@nuxt/ui'

const github = useGithubStore()
const connectOpen = ref(false)

onMounted(() => github.init())

const items = computed<DropdownMenuItem[]>(() => [
  {
    label: github.user?.login ?? '',
    icon: 'i-simple-icons-github',
    type: 'label' as const
  },
  {
    label: 'Sign out',
    icon: 'i-lucide-log-out',
    onSelect: () => github.logout()
  }
])
</script>

<template>
  <template v-if="github.user">
    <UDropdownMenu
      :items="items"
      :content="{ align: 'end' }"
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
      </UButton>
    </UDropdownMenu>
  </template>

  <template v-else>
    <UButton
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
</template>
