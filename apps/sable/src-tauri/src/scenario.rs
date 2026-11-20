use std::fs;
use std::path::PathBuf;
use std::thread;
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::{AppHandle, LogicalSize, Manager, Size};

use crate::host;
use crate::layout;

#[derive(Debug, Serialize)]
struct Report {
    chrome_ready_ms: u64,
    scale_factor: f64,
    checks: Vec<Check>,
}

#[derive(Debug, Serialize)]
struct Check {
    name: String,
    pass: bool,
    detail: String,
}

pub fn run(app: &AppHandle) {
    mark("scenario start");
    let mut checks = Vec::new();
    if let Err(error) = exercise(app, &mut checks) {
        checks.push(Check {
            name: "scenario".into(),
            pass: false,
            detail: error,
        });
    }
    let probe = host::probe(app).ok();
    let report = Report {
        chrome_ready_ms: probe.as_ref().map(|probe| probe.chrome_ready_ms).unwrap_or(0),
        scale_factor: probe.as_ref().map(|probe| probe.scale_factor).unwrap_or(0.0),
        checks,
    };
    let text = serde_json::to_string_pretty(&report).unwrap_or_else(|_| "{}".into());
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../target/sable-scenario.json");
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Err(error) = fs::write(&path, &text) {
        eprintln!("scenario report failed: {error}");
    } else {
        eprintln!("scenario report {}", path.display());
    }
    println!("{text}");
}

fn exercise(app: &AppHandle, checks: &mut Vec<Check>) -> Result<(), String> {
    let ready = wait_until(8_000, || host::probe(app).map(|probe| probe.chrome_ready_ms > 0).unwrap_or(false));
    record(checks, "chrome-webview", ready, "privileged chrome document finished loading");
    let hook = wait_until(3_000, || host::probe(app).map(|probe| probe.hook_ready).unwrap_or(false));
    record(checks, "shortcut-hook", hook, "low-level keyboard hook installed");

    let port = host::probe(app)?.fixture_port;
    let mut ids = Vec::new();
    for index in 1..=3 {
        mark(&format!("fixture {index}"));
        host::new_tab(app)?;
        let id = active_id(app)?;
        host::navigate(app, format!("http://127.0.0.1:{port}/persist.html?n={index}"))?;
        let loaded = wait_title_contains(app, &id, &format!("Persist {index}"), 8_000);
        record(checks, &format!("fixture-{index}"), loaded, if loaded { "local page kept its document" } else { "timed out" });
        host::eval_tab(
            app,
            &id,
            &format!(
                "(() => {{ const note = document.getElementById('note'); if (!note) return; note.value = 'kept-{index}'; note.dispatchEvent(new Event('input', {{bubbles:true}})); }})()"
            ),
        )?;
        ids.push(id);
    }
    host::new_tab(app)?;
    let login = active_id(app)?;
    host::navigate(app, format!("http://127.0.0.1:{port}/login.html"))?;
    let login_loaded = wait_title_contains(app, &login, "Sign in", 8_000);
    record(checks, "login-page", login_loaded, "local sign-in form rendered");
    ids.push(login.clone());

    host::new_tab(app)?;
    let example = active_id(app)?;
    host::navigate(app, "https://example.com".into())?;
    let example_loaded = wait_title_contains(app, &example, "Example", 20_000);
    record(checks, "example.com", example_loaded, host::tab_title(app, &example).unwrap_or_default());
    ids.push(example.clone());

    let mut origins = Vec::new();
    for id in ids.iter().take(3) {
        host::select_tab(app, id.clone())?;
        origins.push(probe_field(app, id, 5_000)?);
    }
    let order = [0usize, 4, 1, 3, 2, 0];
    for index in order {
        host::select_tab(app, ids[index].clone())?;
        thread::sleep(Duration::from_millis(120));
    }
    let mut stable = true;
    let mut detail = String::new();
    for (index, id) in ids.iter().take(3).enumerate() {
        host::select_tab(app, id.clone())?;
        let again = probe_field(app, id, 5_000)?;
        let same = again.origin == origins[index].origin && again.note.as_deref() == Some(&format!("kept-{}", index + 1));
        stable &= same;
        detail.push_str(&format!(
            "tab{} origin {} -> {} note {:?}; ",
            index + 1,
            origins[index].origin,
            again.origin,
            again.note
        ));
    }
    record(checks, "five-tab-switch-without-reload", stable, detail);

    host::select_tab(app, example.clone())?;
    let example_again = probe_field(app, &example, 8_000)?;
    record(
        checks,
        "example-survives-switch",
        example_again.href.contains("example.com"),
        example_again.href,
    );

    host::select_tab(app, login.clone())?;
    let _ = host::eval_tab(app, &login, "document.getElementById('email')?.focus();");
    thread::sleep(Duration::from_millis(200));
    #[cfg(windows)]
    type_text("ada@example.com");
    thread::sleep(Duration::from_millis(250));
    let login_probe = probe_field(app, &login, 5_000)?;
    record(
        checks,
        "login-keyboard-input",
        login_probe.email.as_deref() == Some("ada@example.com") && login_probe.password.as_deref() != Some("submitted"),
        format!("email={:?} focus={} title-not-submitted", login_probe.email, login_probe.focus),
    );

    host::toggle_split(app)?;
    thread::sleep(Duration::from_millis(200));
    let split = host::probe(app)?;
    let split_ok = split.secondary.as_ref().is_some_and(|pane| pane.width > 200.0 && pane.height > 200.0)
        && split.primary.width > 200.0
        && split.secondary.as_ref().is_some_and(|pane| pane.x >= split.primary.x + split.primary.width - 1.0);
    record(
        checks,
        "split-view",
        split_ok,
        format!("primary={}x{} secondary={:?}", split.primary.width, split.primary.height, split.secondary),
    );

    resize_check(app, checks, 1100.0, 700.0)?;
    resize_check(app, checks, 1400.0, 860.0)?;

    host::open_palette(app)?;
    thread::sleep(Duration::from_millis(400));
    let palette = host::probe(app)?;
    let palette_ok = palette.palette_open
        && palette.overlay.is_some()
        && palette.webview_labels.iter().any(|label| label == "overlay");
    record(checks, "overlay-webview", palette_ok, "command palette is a separate child webview");
    host::close_palette(app)?;

    host::clear_chrome_probe(app);
    host::eval_chrome(
        app,
        "(() => { document.documentElement.dataset.clipProbe = '1'; const node = document.querySelector('.clip-probe'); const rect = node.getBoundingClientRect(); document.title = 'sable-chrome-probe:' + JSON.stringify({top:rect.top,bottom:rect.bottom,height:rect.height,innerHeight:window.innerHeight}); })()",
    )?;
    let clip = wait_probe(app, 3_000);
    let clip_ok = clip.as_ref().and_then(|value| value.get("top")).and_then(|value| value.as_f64()).is_some_and(|top| {
        let inner = clip.as_ref().and_then(|value| value.get("innerHeight")).and_then(|value| value.as_f64()).unwrap_or(0.0);
        top + 1.0 >= inner
    });
    record(
        checks,
        "chrome-popup-is-clipped-by-hwnd",
        clip_ok,
        clip.map(|value| value.to_string()).unwrap_or_else(|| "no clip metrics".into()),
    );
    let _ = host::eval_chrome(app, "document.documentElement.dataset.clipProbe = ''; document.title = 'Sable';");

    host::new_tab(app)?;
    let ipc = active_id(app)?;
    host::navigate(app, format!("http://127.0.0.1:{port}/ipc-probe.html"))?;
    let ipc_title = wait_title_prefix(app, &ipc, "ipc:", 8_000).unwrap_or_default();
    record(
        checks,
        "remote-page-ipc-blocked",
        ipc_title.starts_with("ipc:absent") || ipc_title.starts_with("ipc:blocked"),
        ipc_title,
    );

    host::navigate(app, "http://127.0.0.1:1420/".into())?;
    thread::sleep(Duration::from_millis(600));
    host::eval_tab(
        app,
        &ipc,
        "(() => { const bridge = window.__TAURI_INTERNALS__ || window.__TAURI__; if (!bridge || typeof bridge.invoke !== 'function') { document.title = 'shell-in-page:absent'; return; } bridge.invoke('session_snapshot').then(() => { document.title = 'shell-in-page:allowed'; }, (error) => { document.title = 'shell-in-page:blocked ' + (error && error.message ? error.message : String(error)); }); })()",
    )?;
    let shell_title = wait_title_prefix(app, &ipc, "shell-in-page:", 8_000).unwrap_or_default();
    record(
        checks,
        "shell-origin-in-page-webview-blocked",
        shell_title.starts_with("shell-in-page:absent") || shell_title.starts_with("shell-in-page:blocked"),
        shell_title,
    );

    let personal = host::session_dto(app)?;
    let personal_tabs = personal.spaces.iter().find(|space| space.id == personal.active_space_id).map(|space| space.tabs.len()).unwrap_or(0);
    let work = personal.spaces.iter().find(|space| space.name == "Work").map(|space| space.id.clone());
    if let Some(work) = work {
        host::select_space(app, work)?;
        let switched = host::session_dto(app)?;
        let work_tabs = switched.spaces.iter().find(|space| space.name == "Work").map(|space| space.tabs.len()).unwrap_or(0);
        let personal_still = switched.spaces.iter().find(|space| space.name == "Personal").map(|space| space.tabs.len()).unwrap_or(0);
        record(
            checks,
            "spaces-are-independent",
            work_tabs == 1 && personal_still == personal_tabs && personal_tabs > 1,
            format!("personal={personal_still} work={work_tabs}"),
        );
        host::select_space(app, personal.active_space_id)?;
    }

    let mut closed = true;
    for _ in 0..3 {
        host::new_tab(app)?;
        let id = active_id(app)?;
        host::close_tab(app, Some(id.clone()))?;
        let gone = host::probe(app)?.webview_labels.iter().all(|label| label != &id);
        closed &= gone;
    }
    record(checks, "create-close-cleanup", closed, "closed webview labels are gone");

    host::navigate(app, "https://en.wikipedia.org/wiki/Web_browser".into())?;
    let wiki_id = active_id(app)?;
    let wiki = wait_until(25_000, || {
        host::tab_title(app, &wiki_id)
            .map(|title| title.to_lowercase().contains("browser") || title.to_lowercase().contains("wikipedia"))
            .unwrap_or(false)
    });
    record(checks, "wikipedia", wiki, host::tab_title(app, &wiki_id).unwrap_or_default());

    host::select_tab(app, ids[0].clone())?;
    host::toggle_split(app)?;
    thread::sleep(Duration::from_millis(200));
    Ok(())
}

fn resize_check(app: &AppHandle, checks: &mut Vec<Check>, width: f64, height: f64) -> Result<(), String> {
    if let Some(window) = app.get_window("shell") {
        let _ = window.unmaximize();
        let _ = window.set_size(Size::Logical(LogicalSize::new(width, height)));
    }
    let start = Instant::now();
    let mut probe = host::probe(app)?;
    while start.elapsed() < Duration::from_millis(2_000) && (probe.logical_width - width).abs() > 8.0 {
        thread::sleep(Duration::from_millis(50));
        probe = host::probe(app)?;
    }
    let expected = layout::frame(
        probe.logical_width,
        probe.logical_height,
        false,
        probe.secondary.as_ref().map(|_| 0.5),
        false,
    );
    let width_ok = (probe.logical_width - width).abs() < 80.0;
    let matches = (probe.primary.y - expected.primary.y).abs() < 1.5
        && (probe.chrome.height - expected.chrome.height).abs() < 1.5
        && probe.scale_factor > 0.0;
    record(
        checks,
        &format!("resize-{width:.0}x{height:.0}"),
        width_ok && matches,
        format!(
            "scale={} window={}x{} chrome_h={} primary_y={}",
            probe.scale_factor, probe.logical_width, probe.logical_height, probe.chrome.height, probe.primary.y
        ),
    );
    Ok(())
}

#[derive(Debug)]
struct PageProbe {
    href: String,
    origin: f64,
    note: Option<String>,
    email: Option<String>,
    password: Option<String>,
    focus: bool,
}

fn probe_field(app: &AppHandle, id: &str, timeout_ms: u64) -> Result<PageProbe, String> {
    host::eval_tab(
        app,
        id,
        "(() => { const note = document.getElementById('note'); const email = document.getElementById('email'); const password = document.getElementById('password'); const payload = {href: location.href, origin: performance.timeOrigin, note: note ? note.value : null, email: email ? email.value : null, password: password ? password.value : null, focus: document.hasFocus()}; document.title = 'sable-probe:' + JSON.stringify(payload); })()",
    )?;
    let title = wait_title_prefix(app, id, "sable-probe:", timeout_ms).ok_or_else(|| "probe timed out".to_string())?;
    let json = title.trim_start_matches("sable-probe:");
    let value: serde_json::Value = serde_json::from_str(json).map_err(|error| error.to_string())?;
    Ok(PageProbe {
        href: value.get("href").and_then(|item| item.as_str()).unwrap_or("").to_string(),
        origin: value.get("origin").and_then(|item| item.as_f64()).unwrap_or(0.0),
        note: value.get("note").and_then(|item| item.as_str()).map(str::to_string),
        email: value.get("email").and_then(|item| item.as_str()).map(str::to_string),
        password: value.get("password").and_then(|item| item.as_str()).map(str::to_string),
        focus: value.get("focus").and_then(|item| item.as_bool()).unwrap_or(false),
    })
}

fn wait_probe(app: &AppHandle, timeout_ms: u64) -> Option<serde_json::Value> {
    let start = Instant::now();
    while start.elapsed() < Duration::from_millis(timeout_ms) {
        let text = host::chrome_probe(app);
        if !text.is_empty() {
            return serde_json::from_str(&text).ok();
        }
        thread::sleep(Duration::from_millis(40));
    }
    None
}

fn active_id(app: &AppHandle) -> Result<String, String> {
    let session = host::session_dto(app)?;
    session
        .spaces
        .iter()
        .find(|space| space.id == session.active_space_id)
        .map(|space| space.active_tab_id.clone())
        .ok_or_else(|| "no active tab".into())
}

fn wait_title_contains(app: &AppHandle, id: &str, needle: &str, timeout_ms: u64) -> bool {
    wait_until(timeout_ms, || host::tab_title(app, id).map(|title| title.contains(needle)).unwrap_or(false))
}

fn wait_title_prefix(app: &AppHandle, id: &str, prefix: &str, timeout_ms: u64) -> Option<String> {
    let start = Instant::now();
    while start.elapsed() < Duration::from_millis(timeout_ms) {
        if let Ok(title) = host::tab_title(app, id) {
            if title.starts_with(prefix) {
                return Some(title);
            }
        }
        thread::sleep(Duration::from_millis(40));
    }
    host::tab_title(app, id).ok()
}

fn wait_until(timeout_ms: u64, mut predicate: impl FnMut() -> bool) -> bool {
    let start = Instant::now();
    while start.elapsed() < Duration::from_millis(timeout_ms) {
        if predicate() {
            return true;
        }
        thread::sleep(Duration::from_millis(40));
    }
    predicate()
}

fn mark(message: &str) {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../target/sable-boot.log");
    if let Ok(mut file) = fs::OpenOptions::new().create(true).append(true).open(path) {
        use std::io::Write;
        let _ = writeln!(file, "{message}");
    }
}

fn record(checks: &mut Vec<Check>, name: &str, pass: bool, detail: impl Into<String>) {
    mark(&format!("check {name} pass={pass}"));
    checks.push(Check {
        name: name.into(),
        pass,
        detail: detail.into(),
    });
}

#[cfg(windows)]
fn type_text(text: &str) {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId};
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, KEYEVENTF_UNICODE, VIRTUAL_KEY,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        BringWindowToTop, GetForegroundWindow, GetWindowThreadProcessId, SetForegroundWindow,
    };
    unsafe {
        let hwnd = HWND(host::foreground_hwnd() as *mut std::ffi::c_void);
        let foreground = GetForegroundWindow();
        let fg_thread = GetWindowThreadProcessId(foreground, None);
        let ours = GetCurrentThreadId();
        if fg_thread != 0 && fg_thread != ours {
            let _ = AttachThreadInput(fg_thread, ours, true);
        }
        let _ = SetForegroundWindow(hwnd);
        let _ = BringWindowToTop(hwnd);
        if fg_thread != 0 && fg_thread != ours {
            let _ = AttachThreadInput(fg_thread, ours, false);
        }
    }
    thread::sleep(Duration::from_millis(120));
    let mut inputs = Vec::new();
    for unit in text.encode_utf16() {
        inputs.push(key_input(unit, false));
        inputs.push(key_input(unit, true));
    }
    unsafe {
        SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
    }

    fn key_input(scan: u16, up: bool) -> INPUT {
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VIRTUAL_KEY(0),
                    wScan: scan,
                    dwFlags: if up { KEYEVENTF_UNICODE | KEYEVENTF_KEYUP } else { KEYEVENTF_UNICODE },
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        }
    }
}
