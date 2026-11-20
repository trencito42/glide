# Architecture

Sable is one native window with several child WebView2 instances. The React app is the privileged shell. Websites never share that shell's document.

```text
Window "shell"  (decorated Windows window, label is not in any capability)
├── webview "chrome"    toolbar, tabs, spaces, omnibox
├── webview "overlay"   command palette, created only while open
└── webview "tN"        one real page per realized tab
```

## Ownership

| Piece | Owner | Role |
| --- | --- | --- |
| `apps/sable/src-tauri` | Rust | Session, navigation policy, layout, webview lifetime, keyboard hook |
| `apps/sable/src` | React + B4 | Chrome and the new-tab document |
| `vendor/b4` | pinned submodule | Tokens, motion, AnimatedIndicator, inputs, menu primitives |
| WebView2 | Edge runtime | Site rendering, one user-data folder per profile |

`host.rs` is the only module that calls `Window::add_child`. Commands in `lib.rs` are async and hop to `spawn_blocking`, so they are not the UI thread. The low-level keyboard hook only sends an action to a worker. It does not create webviews.

Hidden tabs stay alive, parked off-screen at a 2×2 size. Switching tabs moves that webview back into the content rectangle. It does not navigate again. Restored background tabs are created the first time they are selected.

Split view places two page webviews side by side with a 6px gap. The ratio slider lives in the chrome webview, so the drag is not a third page-area window.

## Trust boundary

Trust is the webview label, not the URL.

`capabilities/chrome.json` lists `webviews: ["chrome", "overlay"]` and does not list `windows`. A page webview whose label is `tN` cannot invoke `session_snapshot` or any other command, including after it is navigated to `http://127.0.0.1:1420/`. The scenario recorded both denials.

`withGlobalTauri` is false. The build script lists commands explicitly, so new commands are not allowed until a capability names them. Page webviews use a profile directory under the app data folder (`profiles/default`, or `profiles/scenario` for the automated run). Chrome and overlay use Tauri's default data directory. They do not share the page profile.

Navigation policy:

- The omnibox upgrades a hostname to `https` and sends other words to DuckDuckGo.
- `javascript:`, `data:`, and `file:` are rejected.
- Page webviews may load `https`, public `http`, the fixture port, and the dev shell origin. The shell origin is still unprivileged because of the label check above.
- Chrome and overlay may load only the app document.
- `window.open` is denied. If the URL passes policy, it is opened as a tab on a background thread.
- Permission requests (camera, microphone, location) are denied in this phase.

Untrusted page script is never evaluated in the chrome document. Title and load callbacks only copy strings into the session model.

## Layout and z-order

Child webviews are sibling HWNDs. A transparent DOM overlay in the chrome document cannot paint over a sibling page webview. The chrome webview is only the toolbar rectangle. The command palette is a separate child webview, about 560×420, created when opened so it is last in z-order. A clip probe is positioned below the chrome document; its measured `top` is past the chrome viewport, which shows that the chrome HWND does not extend over the page.

Logical sizes: horizontal chrome is 104px tall, the sidebar is 252px wide. Windows DPI comes from the window scale factor. On the development monitor that factor was 1.0 (96 DPI).

## B4

The tab strip is not the generic B4 Tabs component. The selected mark is B4 `AnimatedIndicator` with its default travel spring (stiffness 420, damping 38). Icon buttons, the omnibox field, the segmented controls, the slider, and the command palette are B4 primitives. Graphite is the default theme. Porcelain is the alternate. Density is compact. Motion follows the system reduced-motion setting.

## Tradeoffs

- Tauri's multi-webview API is unstable. It is acceptable here because child creation stays off the synchronous UI path and the scenario completed on WebView2 158.
- Emitting from a WebView2 callback deadlocks the UI thread. Session broadcasts are coalesced on a side thread.
- Back and forward are `history.back()` / `history.forward()` in the page, not the native WebView2 GoBack/GoForward APIs. That avoids a `webview2-com` version skew and does not cover history the page itself cannot see.
- One WebView2 user-data folder is shared by every page tab in a profile, which is what makes cookies common across tabs. It also means renderers share a browser process group. A separate folder per tab was tried only as a deadlock experiment and is not the current model.
- The command palette and menus cannot be ordinary DOM popovers over a page. They have to be real windows, or they must stay inside the chrome rectangle.
