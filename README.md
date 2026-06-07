<p align="cente">
  <img src="./public/mainbound-logo.svg" alt="Mainbound" width="88" height="88" />
</p>

<h1 align="center">Mainbound</h1>

<p align="center"><b>From shell to main.</b></p>

<p align="center">
  A macOS terminal workspace that carries your whole workflow toward <code>main</code> —<br/>
  terminal, git, and pull requests together, without leaving the app.
</p>

<p align="center">
  <a href="https://github.com/codewithrafli/mainbound/releases/latest"><img src="https://img.shields.io/github/v/release/codewithrafli/mainbound?style=flat-square&color=8b5cf6" alt="Latest release" /></a>
  <a href="https://github.com/codewithrafli/mainbound/releases"><img src="https://img.shields.io/github/downloads/codewithrafli/mainbound/total?style=flat-square&color=d946ef" alt="Downloads" /></a>
  <img src="https://img.shields.io/badge/platform-macOS-0e0e10?style=flat-square" alt="macOS" />
</p>

<p align="center">
  <img src="./docs/assets/demo.gif" alt="Mainbound demo" width="900" />
</p>

```
Terminal → Changes → Commit → Push → PR → CI → Merge
```

## Why Mainbound?

Shipping a branch means juggling three apps: a terminal for the work, a git
client for the changes, and a browser tab for the pull request. Mainbound
collapses that loop into one window. Every session knows its repository,
branch, and open PR — the cockpit shows changes, ahead/behind, reviews, and
CI without you running a single status command.

## Features

| | |
|---|---|
| **Real terminal, split everything** | zsh over a native PTY (xterm.js + WebGL). Sessions per workspace, recursive splits, drag-and-drop pane docking, ⌘F search, full scrollback that survives view switches and app restarts. |
| **PR Cockpit** | Always-visible strip: branch switcher, local changes, unpushed ↑↓, open PR with review state and CI checks — updates as you work. |
| **File Changes** | Status badges with ±counts, syntax-highlighted diffs, stage/unstage/discard, commit with **AI-generated messages** (local `claude` CLI), history, conflicts. |
| **Pull requests, fully in-app** | Conversation timeline (comments, reviews, commits), inline threads with reply & resolve, per-file diffs, CI checks, merge with confirmation — GitHub website not required. |
| **Notifications** | Bell, OSC 9/777, and a "command finished" heuristic → native banner with a branded chime, in-app toast, and sidebar badges. Claude Code asking for permission in a background session? You'll know. |
| **Secure by design** | GitHub tokens (multi-account) live in the macOS Keychain, never in the webview. Push/pull use your own git credentials. No telemetry. |

## Install

**One line (recommended — no Gatekeeper drama):**

```bash
curl -fsSL https://raw.githubusercontent.com/codewithrafli/mainbound/main/install.sh | bash
```

Or download the `.dmg` from [**Releases**](https://github.com/codewithrafli/mainbound/releases/latest)
(Apple Silicon & Intel). Browser downloads aren't notarized yet, so afterwards run:

```bash
xattr -dr com.apple.quarantine /Applications/Mainbound.app
```

Mainbound updates itself in-app (changelog included) — installing is a one-time affair.

## Keyboard shortcuts

| Shortcut | Action |
| --- | --- |
| ⌘K | Command palette — jump to any session, repo, or action |
| ⌘T | New session |
| ⌘D / ⇧⌘D | Split pane right / down |
| ⌘W | Close focused pane |
| ⌘1 / ⌘2 | Terminal · File Changes |
| ⌘F | Find in terminal |
| ⌘+ / ⌘− / ⌘0 | Zoom |
| ⌘↵ | Commit (from the commit form) |

## Development

```bash
bun install
bun run tauri dev
```

Requires Rust (`rustup`), [Bun](https://bun.sh), and `git` on PATH.

> Note: `src-tauri/.cargo/config.toml` is machine-specific (local cargo
> target dir) and untracked — create your own if you develop from an
> external disk.

Releasing: `bun run release 0.x.y` bumps versions, tags, and pushes — CI
builds, signs, and publishes the update feed.

## Architecture

- `src-tauri/src/pty.rs` — portable-pty sessions; output streamed per session via `pty://data/{id}` events
- `src-tauri/src/git.rs` — shells out to `git` (porcelain v2), no libgit2
- `src-tauri/src/github.rs` — GitHub REST + GraphQL via reqwest; tokens in Keychain (`keyring`)
- `src-tauri/src/ai.rs` — commit/PR message generation through the local `claude` CLI
- `app/` — Nuxt 4 SPA (`ssr: false`), Nuxt UI, Pinia stores

## Philosophy

Small and focused. Every feature must answer one question: *does it speed up
the journey from terminal to a merged PR?* No code editor, no AI chat, no
dashboards — there are better tools for those.
