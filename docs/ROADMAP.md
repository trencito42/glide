# Roadmap

Phase 0 is the native shell: real WebView2 tabs, a B4 chrome, spaces, split view, and a label-based trust boundary. That slice runs. The items below are not implemented.

## Next implementation phase

1. Read child HWND bounds back after layout, and repeat the resize and split checks on a 150% DPI monitor.
2. Stop the scenario from leaving probe JSON in tab titles. Keep the assertions on a side channel.
3. Drive back, forward, and reload in the scenario, then switch those commands to WebView2 GoBack, GoForward, and Reload.
4. Favicons for pinned tabs. The letter mark is the fallback today.
5. Restore the normal profile on launch, including lazy creation of background tabs. The model already skips unrealized tabs until selection. The on-disk session needs a crash-safe write.
6. Downloads need a visible result. The files already land in `%USERPROFILE%\Downloads\Sable`.
7. Permission requests are denied with no prompt. A real prompt is the next security UI, still outside the page webview.

## Later, and not started

Password manager, account sync, extension installation, ad blocking, reader mode, floating video, cloud services, a large settings app, and an auto-updater. None of these have placeholder UI.

## Blockers to watch

- Tauri multi-webview is an unstable API. `Window::add_child` from a synchronous command or a WebView2 callback deadlocks on Windows.
- Sibling webviews cannot be covered by HTML. Any new floating surface needs its own child webview or it must stay inside the chrome rectangle.
- A shared user-data folder is required for normal cookies. Creating a second environment for that folder with different browser arguments is a known WebView2 failure. Sable does not pass extra browser arguments.
- This machine's monitor is 96 DPI, so per-monitor layout is not yet proven.
