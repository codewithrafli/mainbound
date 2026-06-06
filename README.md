# tide

A macOS terminal workspace for parallel coding sessions — inspired by [cmux](https://github.com/manaflow-ai/cmux), built with **Tauri 2 + Nuxt 4 + Nuxt UI**.

## Features

- **Terminal** — real PTY sessions (zsh) rendered with xterm.js (WebGL), multiple sessions per workspace, recursive split panes (right/down) with draggable dividers, sessions survive view switches with full scrollback
- **Workspaces** — add a folder, tide discovers every git repo inside it and shows branches
- **File Changes** — git status with M/U/A/D badges and ±line counts, syntax-highlighted diffs, stage/unstage, commit with summary + description, history, conflict detection
- **GitHub** — push/pull (via your local git credentials), open PR list with CI status, create PRs. Auth via PAT or OAuth device flow; token stored in the macOS Keychain and never exposed to the UI layer

## Development

```bash
bun install
bun run tauri dev
```

Requires Rust (`rustup`), [Bun](https://bun.sh), and `git` on PATH.

> Note: `src-tauri/.cargo/config.toml` redirects build artifacts to a local
> disk path (`~/.cache/cargo-target/tide`) — adjust or delete it on other machines.

## Build

```bash
bun run tauri build   # produces .app + .dmg under the cargo target dir
```

## Keyboard shortcuts

| Shortcut | Action |
| --- | --- |
| ⌘T | New terminal session |
| ⌘D / ⇧⌘D | Split pane right / down |
| ⌘W | Close focused pane |
| ⌘1 / ⌘2 | Terminal / File Changes view |
| ⌘↵ | Commit (when in the commit form) |

## Architecture

- `src-tauri/src/pty.rs` — portable-pty sessions; output streamed per session via `pty://data/{id}` events
- `src-tauri/src/git.rs` — shells out to `git` (porcelain v2), no libgit2
- `src-tauri/src/github.rs` — GitHub REST via reqwest; token in Keychain (`keyring`)
- `src-tauri/src/workspace.rs` / `store.rs` — repo discovery + JSON persistence in app data dir
- `app/` — Nuxt 4 SPA (`ssr: false`), Pinia stores (`terminals`, `workspaces`, `git`, `github`, `ui`)
