# Sable agent notes

Sable is a native Windows browser. Do not replace WebView2 with an iframe, a screenshot, Electron, or a mocked page.

## Boundaries

- Do not edit `vendor/b4`. Rebuild and relink it with `pnpm b4:build` and `pnpm b4:link`.
- Do not add the Search repository as a dependency or copy its branding.
- Do not grant capabilities to the `shell` window. Page webviews inherit a window capability. Privileged commands belong only to the `chrome` and `overlay` webview labels.
- Do not call `Window::add_child` from a synchronous command or from a WebView2 callback. Create child webviews from a background thread. `Window::add_child` then marshals onto the UI thread.
- Do not emit events or evaluate privileged script from a WebView2 navigation, load, or title callback. That re-enters the webview and deadlocks the UI thread. Update state, mark the session dirty, and let the publisher thread emit.

## Checks

```powershell
pnpm typecheck
pnpm test
cargo test --manifest-path apps/sable/src-tauri/Cargo.toml
```

A running `pnpm dev` locks `sable.exe`. Do not kill that process just to rebuild. Native scenario evidence goes to `target/sable-scenario.json` when `SABLE_SCENARIO=1`.

## Product limits for this phase

No password manager, sync, extension installation, ad blocking, reader mode, picture in picture, telemetry, or auto-update. Permission prompts are denied. Downloads, if a site starts one, go to `%USERPROFILE%\Downloads\Sable`.
