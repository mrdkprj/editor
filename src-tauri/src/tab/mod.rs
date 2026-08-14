use serde::{Deserialize, Serialize};
use std::sync::{Mutex, OnceLock};
use tauri::{Emitter, EventTarget, Manager};

#[cfg(target_os = "linux")]
#[path = "gtk.rs"]
pub(crate) mod platform_impl;
#[cfg(target_os = "windows")]
#[path = "windows.rs"]
pub(crate) mod platform_impl;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "name", content = "data", rename_all = "camelCase")]
enum TabEvent {
    Maximized,
    Unmaximized,
    TitleChanged(Title),
    Reordered(Vec<WebviewTitle>),
    Closed(String),
    ModeChanged(ModeChangedArg),
    Added(Title),
    Activated,
    Close(),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "name", content = "data", rename_all = "camelCase")]
pub enum TabRequest {
    Select(String),
    Reorder(Vec<WebviewTitle>),
    CloseAll,
    Cancel,
    Update(WebviewTitle),
    Add,
    ToggleTabMode(bool),
    Detach,
    ToggleMaximize,
    Minimize,
    StartDrag,
    StartResizeDrag(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ModeChangedArg {
    tab_mode: bool,
    tabs: Vec<WebviewTitle>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Title {
    label: String,
    title: String,
    path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WindowMode {
    pub tab_mode: bool,
    pub active_tab_label: String,
    pub close_all: bool,
    pub window_handle: isize,
    pub minimized: bool,
    #[cfg(windows)]
    undecorated_resize: isize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TabState {
    pub tabs: Vec<Tab>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WebviewTitle {
    pub label: String,
    pub title: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct Bounds {
    width: u32,
    height: u32,
    x: i32,
    y: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(crate) struct Tab {
    window_handle: isize,
    pub label: String,
    pub title: String,
    pub path: String,
    bounds: Bounds,
    #[cfg(windows)]
    inset: WindowInset,
    #[cfg(windows)]
    style: isize,
    #[cfg(windows)]
    parent: Option<isize>,
    #[cfg(windows)]
    owner: Option<isize>,
}

impl PartialEq for Tab {
    fn eq(&self, other: &Self) -> bool {
        self.label.eq(&other.label)
    }
}

#[cfg(windows)]
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct WindowInset {
    x: i32,
    y: i32,
}

pub(crate) static HOST: OnceLock<String> = OnceLock::new();

pub fn init(app: &tauri::AppHandle, host: &str) {
    let _ = HOST.set(host.to_string());
    let host = app.get_webview_window(host).unwrap();

    app.manage(Mutex::new(TabState::default()));
    app.manage(Mutex::new(WindowMode::default()));

    #[cfg(windows)]
    platform_impl::prepare(app);

    let cloned = app.clone();
    host.on_window_event(move |e| {
        if let tauri::WindowEvent::Focused(focused) = e {
            let mode = cloned.state::<Mutex<WindowMode>>();
            if let Ok(mode) = mode.try_lock() {
                if mode.tab_mode && *focused {
                    emit_to(&cloned, TabEvent::Activated, &mode.active_tab_label);
                }
            };
        }
    });
}

pub fn update(app: &tauri::AppHandle, label: &str, title: &str, path: &str) {
    platform_impl::update(app, label, title, path);
}

pub fn remove(app: &tauri::AppHandle, label: &str) {
    platform_impl::remove(app, label);
}

fn emit(app: &tauri::AppHandle, event: TabEvent, except: Option<&str>) {
    if let Some(except) = except {
        let _ = app.emit_filter("tab_event", event, |t| match t {
            EventTarget::WebviewWindow {
                label,
            } => label != except,
            _ => false,
        });
    } else {
        let _ = app.emit("tab_event", event);
    }
}

fn emit_to(app: &tauri::AppHandle, event: TabEvent, target: &str) {
    let _ = app.emit_to(
        EventTarget::WebviewWindow {
            label: target.to_string(),
        },
        "tab_event",
        event,
    );
}

pub fn handle_request(window: &tauri::WebviewWindow, req: TabRequest) -> bool {
    match req {
        TabRequest::Add => platform_impl::add(window),
        TabRequest::Cancel => platform_impl::cancel(window.app_handle()),
        TabRequest::Select(label) => platform_impl::select_tab(window.app_handle(), label),
        TabRequest::Reorder(tabs) => platform_impl::reorder_tab(window, tabs),
        TabRequest::CloseAll => platform_impl::close_all(window.app_handle()),
        TabRequest::Update(webview_title) => platform_impl::update(window.app_handle(), &webview_title.label, &webview_title.title, &webview_title.path),
        TabRequest::Detach => platform_impl::detach(window.app_handle(), window.label()),
        TabRequest::ToggleMaximize => platform_impl::toggle_maximize(window.app_handle()),
        TabRequest::Minimize => platform_impl::minimize(window.app_handle()),
        TabRequest::StartDrag => platform_impl::start_drag(window),
        TabRequest::StartResizeDrag(direction) => platform_impl::start_resize_dragging(window, direction),
        TabRequest::ToggleTabMode(tab_mode) => return platform_impl::toggle_tab_mode(window, tab_mode),
    }
    true
}
