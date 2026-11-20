# Sable

Sable is a native Windows 11 browser. The window is a Tauri 2 host. Page content is Microsoft Edge WebView2. The toolbar, tabs, and command palette are a local React application styled with the [B4](https://github.com/trencito42/b4) design system.

Sable is an independent project. [Search](https://github.com/driceroland/Search) is a behavioral reference only. Its source is not a dependency, and its name and icon are not used.

## Requirements

Verified on this machine on 20 November 2026:

- Windows 11 x64
- Rust stable 1.100.0 (`x86_64-pc-windows-msvc`)
- Visual Studio Build Tools 2026, MSVC 14.52, Windows SDK 10.0.26100.0
- WebView2 Runtime (already present; 158.x)
- Node.js 26 and pnpm 12.21.0

Tauri's Windows prerequisites are the MSVC C++ workload and the WebView2 runtime: <https://v2.tauri.app/start/prerequisites/>.

## Setup

```powershell
git submodule update --init vendor/b4
pnpm install
pnpm b4:install
pnpm b4:build
pnpm b4:link
```

B4 stays a pinned submodule (`vendor/b4`, commit `e468efd`). The app does not edit it. `pnpm b4:link` copies the built `@b4/tokens`, `@b4/motion`, and `@b4/react` packages into gitignored `third_party/b4-linked` and rewrites their `workspace:*` dependencies to `file:` links. Nested pnpm workspaces cannot see those `workspace:*` names from the parent, and the submodule is not modified to fix that.

## Commands

```powershell
pnpm dev          # Vite on 127.0.0.1:1420 and the native window
pnpm typecheck    # tsc --noEmit
pnpm test         # Vitest
pnpm check        # typecheck, Vitest, and cargo test
cargo test --manifest-path apps/sable/src-tauri/Cargo.toml
pnpm build        # frontend production build, then tauri build
```

`cargo` must be on `PATH` (`%USERPROFILE%\.cargo\bin`).

Automated native scenario, from the repo root:

```powershell
$env:SABLE_SCENARIO = "1"
pnpm dev
```

The scenario uses a separate WebView2 profile (`profiles/scenario`) and does not overwrite the normal session. It writes `target/sable-scenario.json`. Leave the window open when you are done reading the report; closing the `pnpm dev` terminal closes the browser.

## Current slice

Graphite is the default theme. Porcelain is the other theme. The shell has a compact tab strip, pinned-tab marks, a top or side layout, an omnibox, back/forward/reload, spaces, split view, and a command palette on its own child webview. Shortcuts: Ctrl+L, Ctrl+T, Ctrl+W, Ctrl+Tab, Ctrl+Shift+Tab, Ctrl+K. Escape closes the palette.

See `docs/ARCHITECTURE.md`, `docs/FEASIBILITY.md`, `docs/ROADMAP.md`, and `docs/TESTING.md`.
