pub use crate::manager::tab::TabRequest;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU16, Ordering::Relaxed},
        Mutex,
    },
};
use tauri::{AppHandle, Emitter, EventTarget, Manager, WebviewWindow};
use windows::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, WPARAM},
    UI::{
        Shell::{DefSubclassProc, SetWindowSubclass},
        WindowsAndMessaging::{SIZE_MAXIMIZED, SIZE_RESTORED, WM_SIZE},
    },
};
mod tab;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "name", content = "data", rename_all = "camelCase")]
pub enum ChangeWindowStateRequest {
    ToggleMaximize,
    Minimize,
    UpdateTitle(WindowTitle),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "name", content = "data", rename_all = "camelCase")]
pub enum ChangeWindowStateResult {
    Toggled(Option<Bounds>),
    Minimized(Bounds),
    Maximized,
    Unmaximized,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WindowMode {
    is_dark: bool,
    is_tab_mode: bool,
    active_tab_labels: HashMap<String, String>,
    #[cfg(windows)]
    undecorated_resize: HashMap<String, isize>,
    #[cfg(not(windows))]
    host_signals: HashMap<String, u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(crate) struct WindowLabels {
    pub(crate) labels: HashMap<String, WindowTitle>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WindowTitle {
    pub label: String,
    pub title: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Bounds {
    width: u32,
    height: u32,
    x: i32,
    y: i32,
}

static UUID: AtomicU16 = AtomicU16::new(0);
pub const DEFAULT_WINDOW_LABEL: &str = "View";
const WINDOW_STATE_CHANGE_EVENT: &str = "window-state-changed";

pub fn init(app: &AppHandle) {
    app.manage(Mutex::new(WindowMode::default()));
    app.manage(Mutex::new(WindowLabels::default()));
    tab::init(app, "Main");
    let window = app.get_webview_window(DEFAULT_WINDOW_LABEL).unwrap();
    listen_resize(&window);
}

pub fn change_theme(app: &AppHandle, is_dark: bool) {
    let state = app.state::<Mutex<WindowMode>>();
    let mut state = state.lock().unwrap();
    state.is_dark = is_dark;
}

pub fn is_tab_mode(app: &AppHandle) -> bool {
    let mode = app.state::<Mutex<WindowMode>>();
    let mode = mode.lock().unwrap();
    mode.is_tab_mode()
}

pub fn add_title(app: &tauri::AppHandle, label: &str, title: &str, path: &str) {
    let state = app.state::<Mutex<WindowLabels>>();
    let mut state = state.lock().unwrap();
    state.labels.insert(
        label.to_string(),
        WindowTitle {
            label: label.to_string(),
            title: title.to_string(),
            path: path.to_string(),
        },
    );
}

pub fn change_window_state(window: &WebviewWindow, request: ChangeWindowStateRequest) {
    match request {
        ChangeWindowStateRequest::UpdateTitle(title) => update_title(window.app_handle(), title),
        ChangeWindowStateRequest::ToggleMaximize => toggle_maximize(window),
        ChangeWindowStateRequest::Minimize => minimize(window),
    }
}

fn update_title(app: &AppHandle, title: WindowTitle) {
    let state = app.state::<Mutex<WindowLabels>>();
    let mut state = state.lock().unwrap();
    state.labels.insert(title.label.to_string(), title.clone());
    tab::update(app, &title.label, &title.title, &title.path);
}

fn toggle_maximize(window: &WebviewWindow) {
    let is_tab_mode = {
        let app = window.app_handle();
        let state = app.state::<Mutex<WindowMode>>();
        let state = state.lock().unwrap();
        state.is_tab_mode()
    };

    let result = if is_tab_mode {
        tab::toggle_maximize(window)
    } else {
        if window.is_maximized().unwrap_or_default() {
            let _ = window.unmaximize();
            None
        } else {
            let size = window.inner_size().unwrap();
            let position = window.inner_position().unwrap();
            let _ = window.maximize();
            Some(Bounds {
                width: size.width as _,
                height: size.height as _,
                x: position.x,
                y: position.y,
            })
        }
    };

    let _ = window.emit_to(
        EventTarget::WebviewWindow {
            label: window.label().to_string(),
        },
        WINDOW_STATE_CHANGE_EVENT,
        ChangeWindowStateResult::Toggled(result),
    );
}

fn minimize(window: &tauri::WebviewWindow) {
    let is_tab_mode = {
        let app = window.app_handle();
        let state = app.state::<Mutex<WindowMode>>();
        let state = state.lock().unwrap();
        state.is_tab_mode()
    };

    let result = if is_tab_mode {
        tab::minimize(window)
    } else {
        let size = window.inner_size().unwrap();
        let position = window.inner_position().unwrap();
        let _ = window.minimize();
        Bounds {
            width: size.width as _,
            height: size.height as _,
            x: position.x,
            y: position.y,
        }
    };

    let _ = window.emit_to(
        EventTarget::WebviewWindow {
            label: window.label().to_string(),
        },
        WINDOW_STATE_CHANGE_EVENT,
        ChangeWindowStateResult::Minimized(result),
    );
}

pub fn remove_window(app: &tauri::AppHandle, label: &str) {
    let state = app.state::<Mutex<WindowLabels>>();
    let mut state = state.lock().unwrap();
    state.labels.remove(label);

    /* Remove from tab  */
    tab::remove(app, label);

    if state.labels.is_empty() {
        for (_, host) in app.webview_windows() {
            let _ = host.hide();
            let _ = host.destroy();
        }
    }
}

pub fn on_tab_request(window: &WebviewWindow, request: TabRequest) -> bool {
    tab::handle_request(window, request)
}

pub(crate) fn get_new_window_label() -> String {
    let id = UUID.fetch_add(1, Relaxed);
    format!("{}-{:?}", DEFAULT_WINDOW_LABEL, id)
}

pub(crate) fn create_new_window(app: &AppHandle, label: String) {
    let config = &app.config().app.windows[1];
    let mut config = config.clone();
    config.label = label;

    {
        let mode = app.state::<Mutex<WindowMode>>();
        let mode = mode.lock().unwrap();

        config.theme = if mode.is_dark {
            Some(tauri::Theme::Dark)
        } else {
            Some(tauri::Theme::Light)
        };

        if mode.is_tab_mode() {
            config.focus = false;
        }
    }
    if cfg!(target_os = "windows") {
        /*
           Must be async for tauri::WebviewWindowBuilder::from_config.
           On Windows, this function deadlocks when used in a synchronous command or event handlers, see the Webview2 issue. You should use async commands and separate threads when creating windows.
        */
        let app = app.clone();
        std::thread::spawn(move || {
            app.clone()
                .run_on_main_thread(move || {
                    let window = tauri::WebviewWindowBuilder::from_config(&app, &config).unwrap().build().unwrap();
                    listen_resize(&window);
                })
                .unwrap();
        })
        .join()
        .unwrap();
    } else {
        let window = tauri::WebviewWindowBuilder::from_config(app, &config).unwrap().build().unwrap();
        listen_resize(&window)
    }
}

fn listen_resize(window: &WebviewWindow) {
    let app = window.app_handle().clone();

    let data = WindowStateChangeData {
        app,
        label: window.label().to_string(),
        maximized: window.is_maximized().unwrap_or_default(),
    };
    let _ = unsafe { SetWindowSubclass(window.hwnd().unwrap(), Some(child_window_subclass), window.hwnd().unwrap().0 as usize, Box::into_raw(Box::new(data)) as usize) };
}

pub(crate) fn create_new_host_window(app: &AppHandle, mode: &WindowMode) -> String {
    let id = UUID.fetch_add(1, Relaxed);
    let config = &app.config().app.windows[0];
    let mut config = config.clone();
    let label = format!("{}-{:?}", config.label, id);
    config.label = label.clone();
    config.theme = if mode.is_dark {
        Some(tauri::Theme::Dark)
    } else {
        Some(tauri::Theme::Light)
    };
    tauri::WebviewWindowBuilder::from_config(app, &config).unwrap().build().unwrap();
    label
}

impl WindowMode {
    pub fn can_toggle_mode(&self, new_mode: bool) -> bool {
        self.is_tab_mode != new_mode
    }

    pub fn is_tab_mode(&self) -> bool {
        self.is_tab_mode
    }

    pub fn enter(&mut self) {
        self.is_tab_mode = true;
    }

    pub fn exit(&mut self) {
        self.is_tab_mode = false;
        self.active_tab_labels.clear();
    }

    pub fn remove(&mut self, host_name: &str) {
        self.active_tab_labels.remove(host_name);
        #[cfg(windows)]
        self.undecorated_resize.remove(host_name);
    }

    #[cfg(windows)]
    pub fn get_undecorated_resize(&self, host_name: &str) -> isize {
        *self.undecorated_resize.get(host_name).unwrap()
    }

    #[cfg(windows)]
    pub fn update_undecorated_resize(&mut self, host_name: &str, window_handle: isize) {
        self.undecorated_resize.insert(host_name.to_string(), window_handle);
    }

    pub fn get_active_tab_label(&self, host_name: &str) -> Option<&str> {
        self.active_tab_labels.get(host_name).map(|s| s.as_str())
    }

    pub fn update_active_tab_label(&mut self, host_name: &str, label: &str) {
        self.active_tab_labels.insert(host_name.to_string(), label.to_string());
    }
}

struct WindowStateChangeData {
    app: AppHandle,
    label: String,
    maximized: bool,
}

unsafe extern "system" fn child_window_subclass(hwnd: HWND, umsg: u32, wparam: WPARAM, lparam: LPARAM, _uidsubclass: usize, dwrefdata: usize) -> LRESULT {
    if umsg == WM_SIZE {
        let flag = wparam.0 as u32;
        let item_data_ptr = dwrefdata as *mut WindowStateChangeData;
        let data = &mut *item_data_ptr;
        let maximized = flag == SIZE_MAXIMIZED;
        let should_handle = maximized || (flag == SIZE_RESTORED && data.maximized);
        if should_handle {
            data.maximized = maximized;
            let mode = data.app.state::<Mutex<WindowMode>>();
            if let Ok(mode) = mode.try_lock() {
                if !mode.is_tab_mode {
                    let _ = data.app.emit_to(
                        EventTarget::WebviewWindow {
                            label: data.label.clone(),
                        },
                        WINDOW_STATE_CHANGE_EVENT,
                        if maximized {
                            ChangeWindowStateResult::Maximized
                        } else {
                            ChangeWindowStateResult::Unmaximized
                        },
                    );
                }
            };
        }
    }

    DefSubclassProc(hwnd, umsg, wparam, lparam)
}
