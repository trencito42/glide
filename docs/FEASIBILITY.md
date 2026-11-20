# Feasibility

Date: 20 November 2026. Machine: Windows 11 x64, WebView2 Runtime 158, Tauri 2.12.7, debug build. Monitor DPI was 96 (`GetDpiForWindow`), and Tauri reported `scale_factor` 1.0. These are local prototype results, not product limits.

The automated run is `SABLE_SCENARIO=1 pnpm dev`. The report is `target/sable-scenario.json`. Chrome became ready in **583 ms**. After that run the process tree was **21 processes** and about **1.6 GB** of working set, including the Wikipedia and example.com renderers still alive in the debug session. The Sable process itself was about **34 MB**. A later screenshot shows the same window on a real login page the session navigated to after the script finished.

## Results

| Requirement | Result | Evidence |
| --- | --- | --- |
| Privileged React chrome in the same native window as WebView2 pages | Pass | `chrome-webview` passed. The running window shows the B4 toolbar above the page. |
| At least five tabs switch without reloading | Pass | `five-tab-switch-without-reload`. `performance.timeOrigin` for the three fixture tabs was unchanged, and each kept its note `kept-1` through `kept-3`. example.com also kept its URL. |
| Two pages visible at once | Pass, layout math | `split-view`: primary 637×736 at the content origin and secondary 637×736 starting at x=643. The check uses the same layout function that places the webviews. It does not read the child HWND rectangles back. |
| Keyboard focus and text entry | Pass | `login-keyboard-input`: the local form focused, received `ada@example.com`, and did not submit. |
| Resize, position, clipping | Partial | `resize-1400x860` passed (window 1400×860, chrome height 104, page y 104). `resize-1100x700` failed because the window was still maximized at 1920×1009 when the size was sampled. Clip probe passed: the probe node sat at top 112 while the chrome viewport was 104px tall, so the chrome document does not cover the page. High-DPI was not exercised; this monitor is 100%. |
| Address navigation | Pass | Fixtures, `https://example.com/`, and `https://en.wikipedia.org/wiki/Web_browser` ("Web browser - Wikipedia"). The open window later showed `https://blipmade.com/`. |
| Back, forward, reload | Not in the scenario | The commands evaluate `history.back()`, `history.forward()`, and `location.reload()` in the focused page webview. No automated assertion covers them yet. |
| Create, close, cleanup | Pass | `create-close-cleanup`: three created labels were absent after close. |
| Hostile pages cannot call privileged commands | Pass | `ipc:blocked` on the fixture origin, and `shell-in-page:blocked` after loading `http://127.0.0.1:1420/` inside a page webview. Both denials name capability `chrome-capability`, which lists only `chrome` and `overlay`. |
| Child webviews do not deadlock the window | Pass after a fix | The first run never wrote a report. `SendMessageTimeout` reported the window hung, stuck on the first new tab. Publishing a Tauri event from the WebView2 load/title callback re-entered the webview on the UI thread. Callbacks now only mark the session dirty. A publisher thread emits. The shared page profile then completed the scenario. |
| Native z-order for the command palette | Pass, structural | `overlay-webview` passed: the palette is its own child webview, created when opened. A DOM menu in the chrome document cannot paint over sibling page webviews; the chrome webview is only the toolbar rectangle. A screenshot of the open palette over two live pages was not taken, because the window was already in interactive use. |

## What failed or is unproven

- One programmatic resize did not apply while the window was maximized.
- No 125% or 150% DPI run.
- Split-view evidence is the layout computation plus a single-page screenshot, not a readback of both child HWNDs.
- Back, forward, and reload were not driven by the scenario.
- The scenario writes probe JSON into `document.title`, so those tab labels stay ugly until the next real navigation. That is test instrumentation, and it is visible in the running window.

## Decision

Tauri 2 multi-webview is sufficient for this phase. Child webviews render real sites, survive tab switches, share one page profile, and cannot call the privileged command set. The Windows deadlock is real if webviews are created or scripted from the wrong place; the current call pattern avoids the two cases that hung this machine. A raw WebView2 shell is not required yet. It becomes the next host only if a later phase needs HWND z-order or history behavior Tauri cannot expose.
