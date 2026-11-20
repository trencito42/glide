use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::thread;
use std::time::Duration;

use serde::Serialize;
use tauri::webview::{
    DownloadEvent, NewWindowResponse, PageLoadEvent, PermissionResponse, Webview, WebviewBuilder,
};
use tauri::{AppHandle, Emitter, LogicalPosition, LogicalSize, Manager, Theme, Url, WebviewUrl};
use tauri::window::Window;

use crate::keys::Action;
use crate::layout::{self, Rect};
use crate::model::{self, Session, Tab, NEW_TAB_URL};
use crate::policy;
use crate::AppState;

pub struct Browser {
    pub session: Session,
    pub chrome: Webview,
    pub overlay: Option<Webview>,
    pub webviews: HashMap<String, Webview>,
    pub palette_open: bool,
    pub scenario: bool,
    pub fixture_port: u16,
    pub profile: PathBuf,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Probe {
    pub scale_factor: f64,
    pub logical_width: f64,
    pub logical_height: f64,
    pub hook_ready: bool,
    pub chrome_ready_ms: u64,
    pub fixture_port: u16,
    pub palette_open: bool,
    pub webview_labels: Vec<String>,
    pub chrome: RectDto,
    pub primary: RectDto,
    pub secondary: Option<RectDto>,
    pub overlay: Option<RectDto>,
}

#[derive(Debug, Serialize)]
pub struct RectDto {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl From<Rect> for RectDto {
    fn from(rect: Rect) -> Self {
        Self {
            x: rect.x,
            y: rect.y,
            width: rect.width,
            height: rect.height,
        }
    }
}

pub fn boot(app: &AppHandle) -> Result<(), String> {
    let window = wait_window(app)?;
    remember_hwnd(app, &window);
    let scenario = std::env::var("SABLE_SCENARIO").ok().as_deref() == Some("1");
    let fixture_port = crate::fixtures::start()?;
    let mut session = if scenario { Session::fresh() } else { load_session(app) };
    normalize(&mut session);
    let profile_name = if scenario { "scenario" } else { "default" };
    let profile = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?
        .join("profiles")
        .join(profile_name);
    fs::create_dir_all(&profile).map_err(|error| error.to_string())?;
    let theme = session.theme.clone();
    let chrome = build_shell_webview(app, &window, "chrome", "chrome", &theme, fixture_port)?;
    {
        let state = app.state::<AppState>();
        *state.browser.lock().map_err(|error| error.to_string())? = Some(Browser {
            session,
            chrome,
            overlay: None,
            webviews: HashMap::new(),
            palette_open: false,
            scenario,
            fixture_port,
            profile,
        });
    }
    let active = with_browser(app, |browser| -> Result<String, String> {
        Ok(browser.session.active_space()?.active_tab_id.clone())
    })??;
    ensure_tab(app, &active)?;
    apply_layout(app)?;
    publish(app)?;
    start_publisher(app.clone());
    crate::keys::install(app.clone())?;
    if scenario {
        let handle = app.clone();
        thread::spawn(move || crate::scenario::run(&handle));
    }
    Ok(())
}

pub fn perform(app: AppHandle, action: Action) -> Result<(), String> {
    match action {
        Action::FocusOmnibox => focus_omnibox(&app),
        Action::NewTab => {
            new_tab(&app)?;
            Ok(())
        }
        Action::CloseTab => {
            close_tab(&app, None)?;
            Ok(())
        }
        Action::NextTab => cycle(&app, false),
        Action::PreviousTab => cycle(&app, true),
        Action::Reload => {
            reload(&app)?;
            Ok(())
        }
        Action::Palette => {
            let open = with_browser(&app, |browser| browser.palette_open)?;
            if open {
                close_palette(&app).map(|_| ())
            } else {
                open_palette(&app).map(|_| ())
            }
        }
        Action::Escape => {
            let open = with_browser(&app, |browser| browser.palette_open)?;
            if open { close_palette(&app).map(|_| ()) } else { Ok(()) }
        }
    }
}

pub fn session_dto(app: &AppHandle) -> Result<crate::model::SessionDto, String> {
    with_browser(app, |browser| browser.session.dto(browser.palette_open))
}

pub fn navigate(app: &AppHandle, input: String) -> Result<crate::model::SessionDto, String> {
    let port = with_browser(app, |browser| browser.fixture_port)?;
    let url = policy::resolve_omnibox(&input, port)?;
    let tab_id = with_browser(app, |browser| -> Result<String, String> {
        Ok(browser.session.active_space()?.focused_tab_id().to_string())
    })??;
    ensure_tab(app, &tab_id)?;
    let webview = with_browser(app, |browser| {
        browser
            .webviews
            .get(&tab_id)
            .cloned()
            .ok_or_else(|| "tab webview is missing".to_string())
    })??;
    if url == NEW_TAB_URL {
        let theme = with_browser(app, |browser| {
            if let Some(tab) = find_tab_mut(&mut browser.session, &tab_id) {
                tab.url = NEW_TAB_URL.into();
                tab.title = "New tab".into();
                tab.loading = true;
            }
            browser.session.theme.clone()
        })?;
        webview
            .navigate(shell_url())
            .map_err(|error| error.to_string())?;
        let _ = webview.eval(surface_script("newtab", &theme));
    } else {
        let parsed = Url::parse(&url).map_err(|error| error.to_string())?;
        with_browser(app, |browser| {
            if let Some(tab) = find_tab_mut(&mut browser.session, &tab_id) {
                tab.url = url.clone();
                tab.loading = true;
            }
        })?;
        webview.navigate(parsed).map_err(|error| error.to_string())?;
        trace(&format!("navigated {tab_id} {url}"));
    }
    finish(app)
}

pub fn go_back(app: &AppHandle) -> Result<crate::model::SessionDto, String> {
    history(app, "history.back()")
}

pub fn go_forward(app: &AppHandle) -> Result<crate::model::SessionDto, String> {
    history(app, "history.forward()")
}

pub fn reload(app: &AppHandle) -> Result<crate::model::SessionDto, String> {
    let webview = focused_webview(app)?;
    webview
        .eval("location.reload()")
        .map_err(|error| error.to_string())?;
    finish(app)
}

pub fn new_tab(app: &AppHandle) -> Result<crate::model::SessionDto, String> {
    let id = with_browser(app, |browser| -> Result<String, String> {
        let tab = browser.session.make_tab();
        let id = tab.id.clone();
        let space = browser.session.active_space_mut()?;
        space.tabs.push(tab);
        space.active_tab_id = id.clone();
        space.focused_pane = "primary".into();
        if space.split_tab_id.as_deref() == Some(id.as_str()) {
            space.split_tab_id = None;
        }
        model::partition_pins(&mut space.tabs);
        Ok(id)
    })??;
    ensure_tab(app, &id)?;
    trace(&format!("new tab {id} created"));
    apply_layout(app)?;
    trace(&format!("new tab {id} laid out"));
    focus_omnibox(app)?;
    finish(app)
}

pub fn close_tab(app: &AppHandle, tab_id: Option<String>) -> Result<crate::model::SessionDto, String> {
    let closing = with_browser(app, |browser| -> Result<String, String> {
        let space = browser.session.active_space()?;
        Ok(tab_id.clone().unwrap_or_else(|| space.focused_tab_id().to_string()))
    })??;
    let webview = with_browser(app, |browser| browser.webviews.remove(&closing))?;
    if let Some(webview) = webview {
        let _ = webview.close();
    }
    let replacement = with_browser(app, |browser| {
        let index = browser
            .session
            .active_space()?
            .tabs
            .iter()
            .position(|tab| tab.id == closing);
        if let Some(index) = index {
            browser.session.active_space_mut()?.tabs.remove(index);
        }
        {
            let space = browser.session.active_space_mut()?;
            if space.split_tab_id.as_deref() == Some(closing.as_str()) {
                space.split_tab_id = None;
            }
            space.focused_pane = "primary".into();
        }
        if browser.session.active_space()?.tabs.is_empty() {
            let tab = browser.session.make_tab();
            let id = tab.id.clone();
            let space = browser.session.active_space_mut()?;
            space.tabs.push(tab);
            space.active_tab_id = id.clone();
            return Ok::<_, String>(Some(id));
        }
        let space = browser.session.active_space_mut()?;
        if !space.tabs.iter().any(|tab| tab.id == space.active_tab_id) {
            let next = index.unwrap_or(0).min(space.tabs.len() - 1);
            space.active_tab_id = space.tabs[next].id.clone();
        }
        Ok(Some(space.active_tab_id.clone()))
    })??;
    if let Some(id) = replacement {
        ensure_tab(app, &id)?;
    }
    apply_layout(app)?;
    finish(app)
}

pub fn select_tab(app: &AppHandle, tab_id: String) -> Result<crate::model::SessionDto, String> {
    with_browser(app, |browser| -> Result<(), String> {
        let space = browser.session.active_space_mut()?;
        if space.tab(&tab_id).is_none() {
            return Err("tab not found".to_string());
        }
        if space.split_tab_id.as_deref() == Some(tab_id.as_str()) {
            let previous = space.active_tab_id.clone();
            space.split_tab_id = Some(previous);
            space.focused_pane = "primary".into();
        }
        space.active_tab_id = tab_id.clone();
        if space.split_tab_id.as_deref() == Some(space.active_tab_id.as_str()) {
            space.split_tab_id = None;
        }
        Ok(())
    })??;
    ensure_tab(app, &tab_id)?;
    apply_layout(app)?;
    focus_tab(app, &tab_id)?;
    finish(app)
}

pub fn pin_tab(app: &AppHandle, tab_id: String, pinned: bool) -> Result<crate::model::SessionDto, String> {
    with_browser(app, |browser| -> Result<(), String> {
        let space = browser.session.active_space_mut()?;
        let tab = space.tab_mut(&tab_id).ok_or_else(|| "tab not found".to_string())?;
        tab.pinned = pinned;
        model::partition_pins(&mut space.tabs);
        Ok(())
    })??;
    finish(app)
}

pub fn move_tab(app: &AppHandle, tab_id: String, to_index: usize) -> Result<crate::model::SessionDto, String> {
    with_browser(app, |browser| {
        let space = browser.session.active_space_mut()?;
        model::move_tab(&mut space.tabs, &tab_id, to_index)
    })??;
    finish(app)
}

pub fn set_layout(app: &AppHandle, layout_name: String) -> Result<crate::model::SessionDto, String> {
    if layout_name != "horizontal" && layout_name != "sidebar" {
        return Err("unknown layout".into());
    }
    with_browser(app, |browser| browser.session.layout = layout_name)?;
    apply_layout(app)?;
    finish(app)
}

pub fn set_theme(app: &AppHandle, theme: String) -> Result<crate::model::SessionDto, String> {
    if theme != "graphite" && theme != "porcelain" {
        return Err("unknown theme".into());
    }
    let ids = with_browser(app, |browser| {
        browser.session.theme = theme.clone();
        browser
            .session
            .spaces
            .iter()
            .flat_map(|space| space.tabs.iter())
            .filter(|tab| tab.url == NEW_TAB_URL)
            .map(|tab| tab.id.clone())
            .collect::<Vec<_>>()
    })?;
    let script = format!(
        "document.documentElement.setAttribute('data-b4-theme', {}); window.__SABLE_THEME__ = {}; window.dispatchEvent(new Event('sable-theme'));",
        serde_json::to_string(&theme).unwrap_or_else(|_| "\"graphite\"".into()),
        serde_json::to_string(&theme).unwrap_or_else(|_| "\"graphite\"".into())
    );
    for id in ids {
        if let Ok(webview) = webview_for(app, &id) {
            let _ = webview.eval(&script);
        }
    }
    apply_layout(app)?;
    finish(app)
}

pub fn select_space(app: &AppHandle, space_id: String) -> Result<crate::model::SessionDto, String> {
    let active = with_browser(app, |browser| {
        if !browser.session.spaces.iter().any(|space| space.id == space_id) {
            return Err("space not found".to_string());
        }
        browser.session.active_space_id = space_id;
        Ok(browser.session.active_space()?.active_tab_id.clone())
    })??;
    ensure_tab(app, &active)?;
    apply_layout(app)?;
    finish(app)
}

pub fn new_space(app: &AppHandle) -> Result<crate::model::SessionDto, String> {
    let (space_id, tab_id) = with_browser(app, |browser| -> Result<(String, String), String> {
        let count = browser.session.spaces.len() + 1;
        let space_id = browser.session.make_space(&format!("Space {count}"));
        browser.session.active_space_id = space_id.clone();
        let tab_id = browser.session.active_space()?.active_tab_id.clone();
        Ok((space_id, tab_id))
    })??;
    let _ = space_id;
    ensure_tab(app, &tab_id)?;
    apply_layout(app)?;
    finish(app)
}

pub fn toggle_split(app: &AppHandle) -> Result<crate::model::SessionDto, String> {
    let secondary = with_browser(app, |browser| -> Result<Option<String>, String> {
        let space = browser.session.active_space_mut()?;
        if space.split_tab_id.is_some() {
            space.split_tab_id = None;
            space.focused_pane = "primary".into();
            return Ok(None);
        }
        let current = space.active_tab_id.clone();
        let other = space
            .tabs
            .iter()
            .find(|tab| tab.id != current)
            .map(|tab| tab.id.clone())
            .ok_or_else(|| "Open another tab before splitting.".to_string())?;
        space.split_tab_id = Some(other.clone());
        space.split_ratio = 0.5;
        space.focused_pane = "primary".into();
        Ok(Some(other))
    })??;
    if let Some(id) = secondary {
        ensure_tab(app, &id)?;
    }
    let primary = with_browser(app, |browser| -> Result<String, String> {
        Ok(browser.session.active_space()?.active_tab_id.clone())
    })??;
    ensure_tab(app, &primary)?;
    apply_layout(app)?;
    finish(app)
}

pub fn set_split_ratio(app: &AppHandle, ratio: f64) -> Result<crate::model::SessionDto, String> {
    with_browser(app, |browser| -> Result<(), String> {
        let space = browser.session.active_space_mut()?;
        if space.split_tab_id.is_none() {
            return Err("Split view is off.".to_string());
        }
        space.split_ratio = ratio.clamp(0.25, 0.75);
        Ok(())
    })??;
    apply_layout(app)?;
    finish(app)
}

pub fn focus_pane(app: &AppHandle, pane: String) -> Result<crate::model::SessionDto, String> {
    if pane != "primary" && pane != "secondary" {
        return Err("unknown pane".into());
    }
    let tab_id = with_browser(app, |browser| -> Result<String, String> {
        let space = browser.session.active_space_mut()?;
        if pane == "secondary" && space.split_tab_id.is_none() {
            return Err("Split view is off.".to_string());
        }
        space.focused_pane = pane;
        Ok(space.focused_tab_id().to_string())
    })??;
    focus_tab(app, &tab_id)?;
    finish(app)
}

pub fn open_palette(app: &AppHandle) -> Result<crate::model::SessionDto, String> {
    let exists = with_browser(app, |browser| browser.overlay.is_some())?;
    if !exists {
        let window = wait_window(app)?;
        let theme = with_browser(app, |browser| browser.session.theme.clone())?;
        let port = with_browser(app, |browser| browser.fixture_port)?;
        let overlay = build_shell_webview(app, &window, "overlay", "overlay", &theme, port)?;
        with_browser(app, |browser| browser.overlay = Some(overlay))?;
    }
    with_browser(app, |browser| browser.palette_open = true)?;
    apply_layout(app)?;
    if let Some(overlay) = with_browser(app, |browser| browser.overlay.clone())? {
        let _ = overlay.set_focus();
    }
    finish(app)
}

pub fn close_palette(app: &AppHandle) -> Result<crate::model::SessionDto, String> {
    let overlay = with_browser(app, |browser| {
        browser.palette_open = false;
        browser.overlay.take()
    })?;
    if let Some(overlay) = overlay {
        let _ = overlay.close();
    }
    apply_layout(app)?;
    let _ = focus_chrome(app);
    finish(app)
}

pub fn probe(app: &AppHandle) -> Result<Probe, String> {
    let frame = current_frame(app)?;
    let (width, height, scale) = logical_window(app)?;
    let state = app.state::<AppState>();
    let (labels, palette, port) = with_browser(app, |browser| {
        let mut labels = vec!["chrome".into()];
        if browser.overlay.is_some() {
            labels.push("overlay".into());
        }
        labels.extend(browser.webviews.keys().cloned());
        (labels, browser.palette_open, browser.fixture_port)
    })?;
    Ok(Probe {
        scale_factor: scale,
        logical_width: width,
        logical_height: height,
        hook_ready: state.hook_ready.load(Ordering::Relaxed),
        chrome_ready_ms: state.chrome_ready_ms.load(Ordering::Relaxed),
        fixture_port: port,
        palette_open: palette,
        webview_labels: labels,
        chrome: frame.chrome.into(),
        primary: frame.primary.into(),
        secondary: frame.secondary.map(RectDto::from),
        overlay: frame.overlay.map(RectDto::from),
    })
}

pub fn eval_tab(app: &AppHandle, tab_id: &str, script: &str) -> Result<(), String> {
    let webview = webview_for(app, tab_id)?;
    webview.eval(script).map_err(|error| error.to_string())
}

pub fn eval_chrome(app: &AppHandle, script: &str) -> Result<(), String> {
    let chrome = with_browser(app, |browser| browser.chrome.clone())?;
    chrome.eval(script).map_err(|error| error.to_string())
}

pub fn tab_title(app: &AppHandle, tab_id: &str) -> Result<String, String> {
    with_browser(app, |browser| {
        find_tab(&browser.session, tab_id)
            .map(|tab| tab.title.clone())
            .ok_or_else(|| "tab not found".to_string())
    })?
}

pub fn chrome_probe(app: &AppHandle) -> String {
    app.state::<AppState>()
        .chrome_probe
        .lock()
        .map(|guard| guard.clone())
        .unwrap_or_default()
}

pub fn clear_chrome_probe(app: &AppHandle) {
    if let Ok(mut guard) = app.state::<AppState>().chrome_probe.lock() {
        guard.clear();
    }
}

pub fn foreground_hwnd() -> isize {
    // The hook thread reads this without an AppHandle. The value is mirrored
    // into a process-local atomic by remember_hwnd.
    HWND.load(Ordering::Relaxed)
}

pub fn mark_hook_ready(app: &AppHandle) {
    app.state::<AppState>().hook_ready.store(true, Ordering::Relaxed);
}

pub fn resize(app: &AppHandle) {
    request_layout(app.clone());
}

static HWND: std::sync::atomic::AtomicIsize = std::sync::atomic::AtomicIsize::new(0);
static LAYOUT_QUEUED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static SESSION_DIRTY: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

fn mark_session_dirty() {
    SESSION_DIRTY.store(true, Ordering::Release);
}

fn start_publisher(app: AppHandle) {
    thread::spawn(move || loop {
        thread::sleep(Duration::from_millis(40));
        if SESSION_DIRTY.swap(false, Ordering::AcqRel) {
            let _ = publish(&app);
        }
    });
}

fn request_layout(app: AppHandle) {
    if LAYOUT_QUEUED.swap(true, Ordering::AcqRel) {
        return;
    }
    thread::spawn(move || loop {
        thread::sleep(Duration::from_millis(16));
        LAYOUT_QUEUED.store(false, Ordering::Release);
        let _ = apply_layout(&app);
        if !LAYOUT_QUEUED.swap(true, Ordering::AcqRel) {
            LAYOUT_QUEUED.store(false, Ordering::Release);
            break;
        }
    });
}

fn finish(app: &AppHandle) -> Result<crate::model::SessionDto, String> {
    let dto = with_browser(app, |browser| {
        save_session(app, browser);
        sync_window(app, browser);
        browser.session.dto(browser.palette_open)
    })?;
    publish(app)?;
    Ok(dto)
}

fn publish(app: &AppHandle) -> Result<(), String> {
    let (dto, chrome, overlay) = with_browser(app, |browser| {
        (
            browser.session.dto(browser.palette_open),
            browser.chrome.clone(),
            browser.overlay.clone(),
        )
    })?;
    let _ = chrome.emit("session", &dto);
    if let Some(overlay) = overlay {
        let _ = overlay.emit("session", &dto);
    }
    Ok(())
}

fn focus_omnibox(app: &AppHandle) -> Result<(), String> {
    focus_chrome(app)?;
    let chrome = with_browser(app, |browser| browser.chrome.clone())?;
    let _ = chrome.emit("omnibox-focus", ());
    Ok(())
}

fn focus_chrome(app: &AppHandle) -> Result<(), String> {
    let chrome = with_browser(app, |browser| browser.chrome.clone())?;
    chrome.set_focus().map_err(|error| error.to_string())
}

fn focus_tab(app: &AppHandle, tab_id: &str) -> Result<(), String> {
    let (url, webview) = with_browser(app, |browser| {
        let url = find_tab(&browser.session, tab_id).map(|tab| tab.url.clone());
        let webview = browser.webviews.get(tab_id).cloned();
        (url, webview)
    })?;
    if url.as_deref() == Some(NEW_TAB_URL) {
        return focus_omnibox(app);
    }
    if let Some(webview) = webview {
        webview.set_focus().map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn history(app: &AppHandle, script: &str) -> Result<crate::model::SessionDto, String> {
    let webview = focused_webview(app)?;
    webview.eval(script).map_err(|error| error.to_string())?;
    finish(app)
}

fn focused_webview(app: &AppHandle) -> Result<Webview, String> {
    let id = with_browser(app, |browser| -> Result<String, String> {
        Ok(browser.session.active_space()?.focused_tab_id().to_string())
    })??;
    ensure_tab(app, &id)?;
    webview_for(app, &id)
}

fn webview_for(app: &AppHandle, tab_id: &str) -> Result<Webview, String> {
    with_browser(app, |browser| {
        browser
            .webviews
            .get(tab_id)
            .cloned()
            .ok_or_else(|| "tab webview is missing".to_string())
    })?
}

fn cycle(app: &AppHandle, backward: bool) -> Result<(), String> {
    let next = with_browser(app, |browser| -> Result<Option<String>, String> {
        let space = browser.session.active_space()?;
        Ok(model::cycle_tab(&space.tabs, &space.active_tab_id, backward))
    })??;
    if let Some(id) = next {
        select_tab(app, id)?;
    }
    Ok(())
}

fn ensure_tab(app: &AppHandle, tab_id: &str) -> Result<(), String> {
    let existing = with_browser(app, |browser| browser.webviews.contains_key(tab_id))?;
    if existing {
        return Ok(());
    }
    let (url, theme, profile, port) = with_browser(app, |browser| {
        let url = find_tab(&browser.session, tab_id)
            .map(|tab| tab.url.clone())
            .ok_or_else(|| "tab not found".to_string())?;
        Ok::<_, String>((url, browser.session.theme.clone(), browser.profile.clone(), browser.fixture_port))
    })??;
    let window = wait_window(app)?;
    let webview = build_page_webview(app, &window, tab_id, &url, &theme, profile, port)?;
    with_browser(app, |browser| {
        if let Some(tab) = find_tab_mut(&mut browser.session, tab_id) {
            tab.realized = true;
        }
        browser.webviews.insert(tab_id.to_string(), webview);
    })?;
    Ok(())
}

fn apply_layout(app: &AppHandle) -> Result<(), String> {
    let frame = current_frame(app)?;
    let placements = with_browser(app, |browser| {
        let space = match browser.session.active_space() {
            Ok(space) => space.clone(),
            Err(_) => return Vec::new(),
        };
        let mut placements = vec![(browser.chrome.clone(), frame.chrome)];
        if let Some(overlay) = browser.overlay.clone() {
            placements.push((
                overlay,
                if browser.palette_open {
                    frame.overlay.unwrap_or(layout::PARKED)
                } else {
                    layout::PARKED
                },
            ));
        }
        for tab in &space.tabs {
            let Some(webview) = browser.webviews.get(&tab.id).cloned() else {
                continue;
            };
            let rect = if tab.id == space.active_tab_id {
                frame.primary
            } else if space.split_tab_id.as_deref() == Some(tab.id.as_str()) {
                frame.secondary.unwrap_or(layout::PARKED)
            } else {
                layout::PARKED
            };
            placements.push((webview, rect));
        }
        for (id, webview) in &browser.webviews {
            if space.tabs.iter().all(|tab| &tab.id != id) {
                placements.push((webview.clone(), layout::PARKED));
            }
        }
        placements
    })?;
    for (webview, rect) in placements {
        place(&webview, rect)?;
    }
    Ok(())
}

fn current_frame(app: &AppHandle) -> Result<layout::Frame, String> {
    let (width, height, _) = logical_window(app)?;
    with_browser(app, |browser| {
        let space = browser.session.active_space()?;
        let sidebar = browser.session.layout == "sidebar";
        let split = space.split_tab_id.as_ref().map(|_| space.split_ratio);
        Ok(layout::frame(width, height, sidebar, split, browser.palette_open))
    })?
}

fn logical_window(app: &AppHandle) -> Result<(f64, f64, f64), String> {
    let window = wait_window(app)?;
    let scale = window.scale_factor().map_err(|error| error.to_string())?;
    let scale = if scale <= 0.0 { 1.0 } else { scale };
    let size = window.inner_size().map_err(|error| error.to_string())?;
    Ok((size.width as f64 / scale, size.height as f64 / scale, scale))
}

fn place(webview: &Webview, rect: Rect) -> Result<(), String> {
    webview
        .set_position(tauri::Position::Logical(LogicalPosition::new(rect.x, rect.y)))
        .map_err(|error| error.to_string())?;
    webview
        .set_size(tauri::Size::Logical(LogicalSize::new(rect.width.max(1.0), rect.height.max(1.0))))
        .map_err(|error| error.to_string())?;
    Ok(())
}

fn build_shell_webview(
    app: &AppHandle,
    window: &Window,
    label: &str,
    surface: &str,
    theme: &str,
    _port: u16,
) -> Result<Webview, String> {
    let script = surface_script(surface, theme);
    let builder = WebviewBuilder::new(label, WebviewUrl::App("index.html".into()))
        .initialization_script(&script)
        .devtools(cfg!(debug_assertions))
        .zoom_hotkeys_enabled(false)
        .accept_first_mouse(true)
        .background_color(shell_color(theme))
        .on_navigation(|url| policy::shell_navigation_allowed(url))
        .on_new_window(|_url, _features| NewWindowResponse::Deny)
        .on_document_title_changed({
            let app = app.clone();
            move |webview, title| {
                let label = webview.label().to_string();
                note_title(&app, &label, title);
            }
        })
        .on_page_load({
            let app = app.clone();
            move |webview, payload| {
                let label = webview.label().to_string();
                note_load(&app, &label, payload.event(), payload.url());
            }
        });
    window
        .add_child(
            builder,
            LogicalPosition::new(0.0, 0.0),
            LogicalSize::new(2.0, 2.0),
        )
        .map_err(|error| error.to_string())
}

fn build_page_webview(
    app: &AppHandle,
    window: &Window,
    label: &str,
    url: &str,
    theme: &str,
    profile: PathBuf,
    port: u16,
) -> Result<Webview, String> {
    let internal = url == NEW_TAB_URL;
    let initial = if internal {
        WebviewUrl::App("index.html".into())
    } else {
        WebviewUrl::External(Url::parse(url).map_err(|error| error.to_string())?)
    };
    fs::create_dir_all(&profile).map_err(|error| error.to_string())?;
    trace(&format!("create page webview {label} profile {}", profile.display()));
    let mut builder = WebviewBuilder::new(label, initial)
        .devtools(cfg!(debug_assertions))
        .zoom_hotkeys_enabled(true)
        .accept_first_mouse(true)
        .data_directory(profile)
        .background_color(shell_color(theme))
        .on_navigation(move |url| policy::page_navigation_allowed(url, port))
        .on_new_window({
            let app = app.clone();
            move |url, _features| {
                let target = url.to_string();
                let app = app.clone();
                thread::spawn(move || {
                    if let Ok(parsed) = Url::parse(&target) {
                        if policy::ensure_user_navigation(&parsed, port).is_ok() {
                            let _ = navigate(&app, target);
                        }
                    }
                });
                NewWindowResponse::Deny
            }
        })
        .on_permission_request(|_webview, _kind| PermissionResponse::Deny)
        .on_download(|_webview, event| {
            if let DownloadEvent::Requested { destination, .. } = event {
                if let Ok(home) = std::env::var("USERPROFILE") {
                    let mut dir = PathBuf::from(home);
                    dir.push("Downloads");
                    dir.push("Sable");
                    let _ = fs::create_dir_all(&dir);
                    if let Some(name) = destination.file_name().map(|name| name.to_os_string()) {
                        let safe = PathBuf::from(&name);
                        if safe.components().count() == 1 {
                            *destination = dir.join(safe);
                        }
                    }
                }
            }
            true
        })
        .on_document_title_changed({
            let app = app.clone();
            move |webview, title| {
                let label = webview.label().to_string();
                note_title(&app, &label, title);
            }
        })
        .on_page_load({
            let app = app.clone();
            move |webview, payload| {
                let label = webview.label().to_string();
                note_load(&app, &label, payload.event(), payload.url());
            }
        });
    if internal {
        builder = builder.initialization_script(surface_script("newtab", theme));
    }
    let webview = window
        .add_child(
            builder,
            LogicalPosition::new(layout::PARKED.x, layout::PARKED.y),
            LogicalSize::new(layout::PARKED.width, layout::PARKED.height),
        )
        .map_err(|error| error.to_string());
    trace(&format!("page webview {label} {:?}", webview.as_ref().map(|view| view.label().to_string()).map_err(|error| error.clone())));
    webview
}

fn trace(message: &str) {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../target/sable-boot.log");
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
        use std::io::Write;
        let _ = writeln!(file, "{message}");
    }
}

fn note_title(app: &AppHandle, label: &str, title: String) {
    if label == "chrome" || label == "overlay" {
        if let Some(rest) = title.strip_prefix("sable-chrome-probe:") {
            if let Ok(mut guard) = app.state::<AppState>().chrome_probe.lock() {
                *guard = rest.to_string();
            }
        }
        return;
    }
    let changed = with_browser(app, |browser| {
        if let Some(tab) = find_tab_mut(&mut browser.session, label) {
            if tab.title != title {
                tab.title = title;
                return true;
            }
        }
        false
    });
    if changed.unwrap_or(false) {
        // The title callback runs on the WebView2 UI thread. Emitting from
        // there calls back into the webview and deadlocks the window.
        mark_session_dirty();
    }
}

fn note_load(app: &AppHandle, label: &str, event: PageLoadEvent, url: &Url) {
    if label == "chrome" && matches!(event, PageLoadEvent::Finished) {
        let state = app.state::<AppState>();
        if state.chrome_ready_ms.load(Ordering::Relaxed) == 0 {
            let millis = state.started.elapsed().as_millis() as u64;
            state.chrome_ready_ms.store(millis.max(1), Ordering::Relaxed);
        }
        return;
    }
    if label == "chrome" || label == "overlay" {
        return;
    }
    let loading = matches!(event, PageLoadEvent::Started);
    let navigated = url.to_string();
    let changed = with_browser(app, |browser| {
        let Some(tab) = find_tab_mut(&mut browser.session, label) else {
            return false;
        };
        tab.loading = loading;
        if !(tab.url == NEW_TAB_URL && policy::is_shell_document(url)) {
            tab.url = navigated;
        }
        true
    });
    if changed.unwrap_or(false) && !loading {
        mark_session_dirty();
    }
}

fn shell_url() -> Url {
    if cfg!(debug_assertions) {
        Url::parse("http://127.0.0.1:1420/").expect("dev url")
    } else {
        Url::parse("http://tauri.localhost/").expect("shell url")
    }
}

fn surface_script(surface: &str, theme: &str) -> String {
    format!(
        "window.__SABLE_SURFACE__={};window.__SABLE_THEME__={};",
        serde_json::to_string(surface).unwrap_or_else(|_| "\"chrome\"".into()),
        serde_json::to_string(theme).unwrap_or_else(|_| "\"graphite\"".into())
    )
}

fn shell_color(theme: &str) -> tauri::webview::Color {
    if theme == "porcelain" {
        tauri::webview::Color(251, 250, 248, 255)
    } else {
        tauri::webview::Color(10, 11, 13, 255)
    }
}

fn sync_window(app: &AppHandle, browser: &Browser) {
    let Some(window) = app.get_window("shell") else {
        return;
    };
    let title = browser
        .session
        .active_space()
        .ok()
        .and_then(|space| space.tab(space.focused_tab_id()))
        .map(|tab| format!("{} — Sable", tab.title))
        .unwrap_or_else(|| "Sable".into());
    let _ = window.set_title(&title);
    let theme = if browser.session.theme == "porcelain" {
        Theme::Light
    } else {
        Theme::Dark
    };
    let _ = window.set_theme(Some(theme));
}

fn find_tab<'a>(session: &'a Session, id: &str) -> Option<&'a Tab> {
    session.spaces.iter().find_map(|space| space.tab(id))
}

fn find_tab_mut<'a>(session: &'a mut Session, id: &str) -> Option<&'a mut Tab> {
    session.spaces.iter_mut().find_map(|space| space.tab_mut(id))
}

fn normalize(session: &mut Session) {
    if session.theme != "porcelain" {
        session.theme = "graphite".into();
    }
    if session.layout != "sidebar" {
        session.layout = "horizontal".into();
    }
    if session.spaces.is_empty() {
        *session = Session::fresh();
        return;
    }
    if !session.spaces.iter().any(|space| space.id == session.active_space_id) {
        session.active_space_id = session.spaces[0].id.clone();
    }
    for space in &mut session.spaces {
        if space.tabs.is_empty() {
            continue;
        }
        model::partition_pins(&mut space.tabs);
        if space.tab(&space.active_tab_id).is_none() {
            space.active_tab_id = space.tabs[0].id.clone();
        }
        if space.focused_pane != "secondary" {
            space.focused_pane = "primary".into();
        }
        space.split_ratio = space.split_ratio.clamp(0.25, 0.75);
    }
}

fn load_session(app: &AppHandle) -> Session {
    let Ok(path) = session_path(app) else {
        return Session::fresh();
    };
    let Ok(text) = fs::read_to_string(path) else {
        return Session::fresh();
    };
    serde_json::from_str(&text).unwrap_or_else(|_| Session::fresh())
}

fn save_session(app: &AppHandle, browser: &Browser) {
    if browser.scenario {
        return;
    }
    let Ok(path) = session_path(app) else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(text) = serde_json::to_string_pretty(&browser.session) {
        let _ = fs::write(path, text);
    }
}

fn session_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app.path().app_data_dir().map_err(|error| error.to_string())?.join("session.json"))
}

fn wait_window(app: &AppHandle) -> Result<Window, String> {
    let start = std::time::Instant::now();
    while start.elapsed() < Duration::from_secs(5) {
        if let Some(window) = app.get_window("shell") {
            return Ok(window);
        }
        thread::sleep(Duration::from_millis(20));
    }
    Err("The shell window was not created.".into())
}

fn remember_hwnd(app: &AppHandle, window: &Window) {
    #[cfg(windows)]
    {
        if let Ok(hwnd) = window.hwnd() {
            let value = hwnd.0 as isize;
            HWND.store(value, Ordering::Relaxed);
            app.state::<AppState>().hwnd.store(value, Ordering::Relaxed);
        }
    }
    #[cfg(not(windows))]
    {
        let _ = (app, window);
    }
}

fn with_browser<T>(app: &AppHandle, f: impl FnOnce(&mut Browser) -> T) -> Result<T, String> {
    let state = app.state::<AppState>();
    let start = std::time::Instant::now();
    loop {
        {
            let mut guard = state.browser.lock().map_err(|error| error.to_string())?;
            if let Some(browser) = guard.as_mut() {
                return Ok(f(browser));
            }
        }
        if start.elapsed() > Duration::from_secs(8) {
            return Err("Sable is still starting.".into());
        }
        thread::sleep(Duration::from_millis(20));
    }
}
