const LANGS: Record<string, string> = {
  ts: 'typescript', tsx: 'tsx', js: 'javascript', jsx: 'jsx', mjs: 'javascript',
  vue: 'vue', css: 'css', scss: 'scss', html: 'html', json: 'json',
  md: 'markdown', rs: 'rust', py: 'python', go: 'go', sh: 'bash',
  yml: 'yaml', yaml: 'yaml', toml: 'ini', sql: 'sql'
}

export function diffLang(path: string): string {
  const ext = path.split('.').pop()?.toLowerCase() ?? ''
  return LANGS[ext] ?? 'plaintext'
}
