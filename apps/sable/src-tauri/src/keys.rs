use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc::{self, Sender};
use std::thread;

use tauri::AppHandle;

#[derive(Debug, Clone, Copy)]
pub enum Action {
    FocusOmnibox,
    NewTab,
    CloseTab,
    NextTab,
    PreviousTab,
    Palette,
    Escape,
    Reload,
}

static HELD: AtomicU32 = AtomicU32::new(0);

pub fn install(app: AppHandle) -> Result<(), String> {
    let (tx, rx) = mpsc::channel::<Action>();
    let worker = app.clone();
    thread::spawn(move || {
        while let Ok(action) = rx.recv() {
            if let Err(error) = crate::host::perform(worker.clone(), action) {
                eprintln!("shortcut failed: {error}");
            }
        }
    });
    thread::spawn(move || hook_thread(app, tx));
    Ok(())
}

#[cfg(windows)]
fn hook_thread(app: AppHandle, tx: Sender<Action>) {
    use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
    use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_CONTROL, VK_MENU, VK_SHIFT};
    use windows::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, DispatchMessageW, GetForegroundWindow, GetMessageW, SetWindowsHookExW, TranslateMessage,
        HHOOK, KBDLLHOOKSTRUCT, MSG, WH_KEYBOARD_LL,
    };

    unsafe extern "system" fn proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        if code == 0 {
            let info = unsafe { &*(lparam.0 as *const KBDLLHOOKSTRUCT) };
            if handle_key(info.vkCode, info.flags.0) {
                return LRESULT(1);
            }
        }
        unsafe { CallNextHookEx(None, code, wparam, lparam) }
    }

    fn down(vk: u16) -> bool {
        unsafe { GetAsyncKeyState(i32::from(vk)) < 0 }
    }

    fn handle_key(vk: u32, flags: u32) -> bool {
        const UP: u32 = 0x80;
        let up = flags & UP != 0;
        if up {
            if HELD.load(Ordering::Relaxed) == vk {
                HELD.store(0, Ordering::Relaxed);
            }
            return false;
        }
        let ctrl = down(VK_CONTROL.0);
        let shift = down(VK_SHIFT.0);
        let alt = down(VK_MENU.0);
        if alt || !our_window() {
            return false;
        }
        let action = if ctrl {
            match vk {
                0x4C => Some(Action::FocusOmnibox),
                0x54 => Some(Action::NewTab),
                0x57 => Some(Action::CloseTab),
                0x4B => Some(Action::Palette),
                0x52 => Some(Action::Reload),
                0x09 => Some(if shift { Action::PreviousTab } else { Action::NextTab }),
                _ => None,
            }
        } else if vk == 0x1B {
            Some(Action::Escape)
        } else {
            None
        };
        let Some(action) = action else {
            return false;
        };
        if vk != 0x09 && HELD.load(Ordering::Relaxed) == vk {
            return true;
        }
        if vk != 0x09 {
            HELD.store(vk, Ordering::Relaxed);
        }
        if let Some(tx) = sender() {
            let _ = tx.send(action);
        }
        true
    }

    fn our_window() -> bool {
        let wanted = crate::host::foreground_hwnd();
        if wanted == 0 {
            return false;
        }
        let current = unsafe { GetForegroundWindow() };
        current.0 as isize == wanted
    }

    static TX: std::sync::OnceLock<Sender<Action>> = std::sync::OnceLock::new();
    fn sender() -> Option<&'static Sender<Action>> {
        TX.get()
    }

    if TX.set(tx).is_err() {
        eprintln!("shortcut hook already installed");
        return;
    }
    unsafe {
        let hook = SetWindowsHookExW(WH_KEYBOARD_LL, Some(proc), None, 0);
        match hook {
            Ok(handle) => {
                crate::host::mark_hook_ready(&app);
                let _keep: HHOOK = handle;
                let mut message = MSG::default();
                while GetMessageW(std::ptr::addr_of_mut!(message), None, 0, 0) != false {
                    let _ = TranslateMessage(&message);
                    DispatchMessageW(&message);
                }
            }
            Err(error) => eprintln!("shortcut hook failed: {error}"),
        }
    }
}

#[cfg(not(windows))]
fn hook_thread(_app: AppHandle, _tx: Sender<Action>) {}
