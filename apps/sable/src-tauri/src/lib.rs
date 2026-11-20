mod fixtures;
mod host;
mod keys;
mod layout;
mod model;
mod policy;
mod scenario;

use std::sync::atomic::{AtomicBool, AtomicIsize, AtomicU64};
use std::sync::Mutex;
use std::thread;
use std::time::Instant;

use tauri::{AppHandle, Theme, WindowEvent};

pub struct AppState {
    pub started: Instant,
    pub browser: Mutex<Option<host::Browser>>,
    pub chrome_probe: Mutex<String>,
    pub hwnd: AtomicIsize,
    pub hook_ready: AtomicBool,
    pub chrome_ready_ms: AtomicU64,
}

#[tauri::command]
async fn session_snapshot(app: AppHandle) -> Result<model::SessionDto, String> {
    off_ui(move || host::session_dto(&app)).await
}

#[tauri::command]
async fn navigate(app: AppHandle, input: String) -> Result<model::SessionDto, String> {
    off_ui(move || host::navigate(&app, input)).await
}

#[tauri::command]
async fn go_back(app: AppHandle) -> Result<model::SessionDto, String> {
    off_ui(move || host::go_back(&app)).await
}

#[tauri::command]
async fn go_forward(app: AppHandle) -> Result<model::SessionDto, String> {
    off_ui(move || host::go_forward(&app)).await
}

#[tauri::command]
async fn reload(app: AppHandle) -> Result<model::SessionDto, String> {
    off_ui(move || host::reload(&app)).await
}

#[tauri::command]
async fn new_tab(app: AppHandle) -> Result<model::SessionDto, String> {
    off_ui(move || host::new_tab(&app)).await
}

#[tauri::command]
async fn close_tab(app: AppHandle, tab_id: Option<String>) -> Result<model::SessionDto, String> {
    off_ui(move || host::close_tab(&app, tab_id)).await
}

#[tauri::command]
async fn select_tab(app: AppHandle, tab_id: String) -> Result<model::SessionDto, String> {
    off_ui(move || host::select_tab(&app, tab_id)).await
}

#[tauri::command]
async fn pin_tab(app: AppHandle, tab_id: String, pinned: bool) -> Result<model::SessionDto, String> {
    off_ui(move || host::pin_tab(&app, tab_id, pinned)).await
}

#[tauri::command]
async fn move_tab(app: AppHandle, tab_id: String, to_index: usize) -> Result<model::SessionDto, String> {
    off_ui(move || host::move_tab(&app, tab_id, to_index)).await
}

#[tauri::command]
async fn set_layout(app: AppHandle, layout: String) -> Result<model::SessionDto, String> {
    off_ui(move || host::set_layout(&app, layout)).await
}

#[tauri::command]
async fn set_theme(app: AppHandle, theme: String) -> Result<model::SessionDto, String> {
    off_ui(move || host::set_theme(&app, theme)).await
}

#[tauri::command]
async fn select_space(app: AppHandle, space_id: String) -> Result<model::SessionDto, String> {
    off_ui(move || host::select_space(&app, space_id)).await
}

#[tauri::command]
async fn new_space(app: AppHandle) -> Result<model::SessionDto, String> {
    off_ui(move || host::new_space(&app)).await
}

#[tauri::command]
async fn toggle_split(app: AppHandle) -> Result<model::SessionDto, String> {
    off_ui(move || host::toggle_split(&app)).await
}

#[tauri::command]
async fn set_split_ratio(app: AppHandle, ratio: f64) -> Result<model::SessionDto, String> {
    off_ui(move || host::set_split_ratio(&app, ratio)).await
}

#[tauri::command]
async fn focus_pane(app: AppHandle, pane: String) -> Result<model::SessionDto, String> {
    off_ui(move || host::focus_pane(&app, pane)).await
}

#[tauri::command]
async fn open_palette(app: AppHandle) -> Result<model::SessionDto, String> {
    off_ui(move || host::open_palette(&app)).await
}

#[tauri::command]
async fn close_palette(app: AppHandle) -> Result<model::SessionDto, String> {
    off_ui(move || host::close_palette(&app)).await
}

#[tauri::command]
async fn probe(app: AppHandle) -> Result<host::Probe, String> {
    off_ui(move || host::probe(&app)).await
}

async fn off_ui<T: Send + 'static>(work: impl FnOnce() -> Result<T, String> + Send + 'static) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(work)
        .await
        .map_err(|error| error.to_string())?
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let state = AppState {
        started: Instant::now(),
        browser: Mutex::new(None),
        chrome_probe: Mutex::new(String::new()),
        hwnd: AtomicIsize::new(0),
        hook_ready: AtomicBool::new(false),
        chrome_ready_ms: AtomicU64::new(0),
    };
    tauri::Builder::default()
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            session_snapshot,
            navigate,
            go_back,
            go_forward,
            reload,
            new_tab,
            close_tab,
            select_tab,
            pin_tab,
            move_tab,
            set_layout,
            set_theme,
            select_space,
            new_space,
            toggle_split,
            set_split_ratio,
            focus_pane,
            open_palette,
            close_palette,
            probe
        ])
        .setup(|app| {
            let theme = if std::env::var("SABLE_SCENARIO").ok().as_deref() == Some("1") {
                Theme::Dark
            } else {
                Theme::Dark
            };
            let window = tauri::window::WindowBuilder::new(app, "shell")
                .title("Sable")
                .inner_size(1280.0, 840.0)
                .min_inner_size(880.0, 560.0)
                .resizable(true)
                .maximizable(true)
                .minimizable(true)
                .closable(true)
                .decorations(true)
                .theme(Some(theme))
                .build()?;
            let handle = app.handle().clone();
            window.on_window_event(move |event| {
                if matches!(event, WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. }) {
                    host::resize(&handle);
                }
            });
            let handle = app.handle().clone();
            thread::spawn(move || {
                if let Err(error) = host::boot(&handle) {
                    eprintln!("boot failed: {error}");
                }
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("Sable failed to start");
}
