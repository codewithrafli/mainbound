<script setup lang="ts">
import { marked } from 'marked'
import DOMPurify from 'dompurify'

const props = defineProps<{ source: string }>()

const html = computed(() => {
  const raw = marked.parse(props.source, { gfm: true, breaks: true, async: false }) as string
  // GitHub-sourced content is untrusted — sanitize before v-html
  return DOMPurify.sanitize(raw)
})

// links open in the system browser, never navigate the webview
async function onClick(event: MouseEvent) {
  const anchor = (event.target as HTMLElement).closest('a')
  if (!anchor?.href) return
  event.preventDefault()
  const { openUrl } = await import('@tauri-apps/plugin-opener')
  await openUrl(anchor.href).catch(() => {})
}
</script>

<template>
  <!-- content is sanitized with DOMPurify before rendering -->
  <!-- eslint-disable vue/no-v-html -->
  <div
    class="md-body text-[12px] text-toned leading-relaxed break-words"
    @click="onClick"
    v-html="html"
  />
  <!-- eslint-enable vue/no-v-html -->
</template>

<style>
.md-body > :first-child { margin-top: 0; }
.md-body > :last-child { margin-bottom: 0; }
.md-body p { margin: 0.4em 0; }
.md-body h1, .md-body h2, .md-body h3, .md-body h4 {
  color: var(--ui-text-highlighted);
  font-weight: 600;
  margin: 0.9em 0 0.35em;
}
.md-body h1 { font-size: 1.25em; }
.md-body h2 { font-size: 1.15em; padding-bottom: 0.2em; border-bottom: 1px solid var(--ui-border); }
.md-body h3 { font-size: 1.05em; }
.md-body ul, .md-body ol { margin: 0.4em 0; padding-left: 1.4em; }
.md-body ul { list-style: disc; }
.md-body ol { list-style: decimal; }
.md-body li { margin: 0.15em 0; }
.md-body li > ul, .md-body li > ol { margin: 0.1em 0; }
.md-body strong { color: var(--ui-text-highlighted); font-weight: 600; }
.md-body a { color: #58a6ff; }
.md-body a:hover { text-decoration: underline; }
.md-body code {
  font-family: ui-monospace, "SF Mono", Menlo, monospace;
  font-size: 0.92em;
  background: var(--ui-bg-elevated);
  border: 1px solid var(--ui-border);
  border-radius: 4px;
  padding: 0.1em 0.35em;
}
.md-body pre {
  background: var(--ui-bg-elevated);
  border: 1px solid var(--ui-border);
  border-radius: 6px;
  padding: 0.6em 0.8em;
  margin: 0.5em 0;
  overflow-x: auto;
}
.md-body pre code { background: none; border: none; padding: 0; }
.md-body blockquote {
  border-left: 3px solid var(--ui-border-accented);
  color: var(--ui-text-muted);
  padding-left: 0.8em;
  margin: 0.5em 0;
}
.md-body hr { border-color: var(--ui-border); margin: 0.8em 0; }
.md-body table { border-collapse: collapse; margin: 0.5em 0; }
.md-body th, .md-body td {
  border: 1px solid var(--ui-border);
  padding: 0.25em 0.6em;
}
.md-body th { background: var(--ui-bg-elevated); color: var(--ui-text-highlighted); }
.md-body img { max-width: 100%; border-radius: 6px; }
</style>
