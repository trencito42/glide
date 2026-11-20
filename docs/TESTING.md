# Testing

## Automated

```powershell
pnpm typecheck
pnpm test
cargo test --manifest-path apps/sable/src-tauri/Cargo.toml
```

On 20 November 2026: `tsc` passed, Vitest passed 3 tests, and `cargo test --lib` passed 11 tests (layout, tab model, navigation policy).

Native scenario:

```powershell
$env:SABLE_SCENARIO = "1"
pnpm dev
```

Wait for `scenario report` in the console. Read `target/sable-scenario.json`. The script loads local fixtures, `example.com`, and a Wikipedia article, types into a local login form without submitting, switches five tabs, opens a split view, resizes the window, opens the palette webview, and checks that page webviews cannot call `session_snapshot`. It uses `profiles/scenario` and does not replace the normal session file.

Local fixtures are served from `fixtures/sites/` on an ephemeral `127.0.0.1` port:

- `persist.html` records a note and `performance.timeOrigin`
- `login.html` has email and password fields and blocks submit
- `ipc-probe.html` tries the privileged command

## Manual Windows checklist

Do this in `pnpm dev` without `SABLE_SCENARIO`, so the normal profile is used.

- The window opens with a decorated title bar. Resize it from a corner, then snap it to a screen edge and restore it.
- New tab, close, and Ctrl+T / Ctrl+W agree. Middle-click closes a tab. Closing the last tab leaves a new tab.
- Ctrl+Tab and Ctrl+Shift+Tab move the selection. The selected pill travels. With reduced motion enabled in Windows, it does not travel.
- Ctrl+L focuses the omnibox. Enter on `example.com` loads `https://example.com/`. Enter on two words opens a DuckDuckGo search. `javascript:` and `file:` do nothing useful.
- Back, forward, and reload affect the focused page.
- Pin a tab. It stays in the pinned group and shows a letter. Drag a tab within its group.
- Switch Top and Side. Switch Graphite and Porcelain. The page webviews stay where the chrome is not.
- Create a second space. Its tab list is not the first space's list. Switch back and confirm the first list is intact.
- Open two pages, then split. Both documents stay visible. Move the ratio slider. Focus Left and Right.
- Ctrl+K opens the palette over the pages, not under them. Escape closes it. Choosing a tab selects it.
- Click a text field on a page and type. Click the omnibox afterward and confirm page keys no longer edit the omnibox.
- Open a login form. Do not submit credentials. Confirm the form is a real page, not the new-tab document.
- Watch Task Manager. Closing a tab should drop that webview. The privileged commands stay unavailable to the site (the scenario already asserts this).
