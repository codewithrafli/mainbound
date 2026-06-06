# Mainbound

> **From shell to main.**

A macOS terminal workspace that carries your whole workflow toward `main`:

```
Terminal → Changes → Commit → Push → PR → CI → Merge
```

Built with **Tauri 2 + Nuxt 4 + Nuxt UI**.

## Features

- **Terminal** — real PTY sessions (zsh) rendered with xterm.js (WebGL), multiple sessions per workspace, recursive split panes (right/down) with draggable dividers, sessions survive view switches with full scrollback
- **Workspaces** — add a folder, Mainbound discovers every git repo inside it and shows branches
- **File Changes** — git status with M/U/A/D badges and ±line counts, syntax-highlighted diffs, stage/unstage, commit with summary + description, history, conflict detection
- **GitHub** — push/pull (via your local git credentials), open PR list with CI status, create PRs. Auth via PAT or OAuth device flow; token stored in the macOS Keychain and never exposed to the UI layer

## Install

**Recommended — one line, no Gatekeeper drama:**

```bash
curl -fsSL https://raw.githubusercontent.com/codewithrafli/mainbound/main/install.sh | bash
```

Or grab the `.dmg` from [**Releases**](https://github.com/codewithrafli/mainbound/releases/latest)
(Apple Silicon & Intel). Browser downloads aren't notarized yet, so after
installing run:

```bash
xattr -dr com.apple.quarantine /Applications/Mainbound.app
```

Mainbound checks for updates automatically and installs them in-app
(menu **Mainbound → Check for Updates…**), with the changelog shown
before you confirm.

## Development

```bash
bun install
bun run tauri dev
```

Requires Rust (`rustup`), [Bun](https://bun.sh), and `git` on PATH.

> Note: `src-tauri/.cargo/config.toml` redirects build artifacts to a local
> disk path (`~/.cache/cargo-target/mainbound`) — adjust or delete it on other machines.

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
