use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{Mutex, OnceLock},
};
use tauri::{Emitter, EventTarget, Manager};

#[cfg(target_os = "linux")]
#[path = "gtk.rs"]
mod platform_impl;
#[cfg(target_os = "windows")]
#[path = "windows.rs"]
mod platform_impl;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "name", content = "data", rename_all = "camelCase")]
enum TabEvent {
    Maximized,
    Unmaximized,
    TitleChanged(WebviewTitle),
    Reordered(Vec<WebviewTitle>),
    Closed(String),
    ModeChanged(ModeChangedArg),
    Added(WebviewTitle),
    Attached(Vec<WebviewTitle>),
    Activated,
    Close,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "name", content = "data", rename_all = "camelCase")]
pub enum TabRequest {
    Select(String),
    SelectNext,
    SelectPrevious,
    Reorder(Vec<WebviewTitle>),
    CloseAll,
    Cancel,
    Update(WebviewTitle),
    Add(AddTabRequest),
    ToggleTabMode(ToggleTabModeRequest),
    Close,
    Attach(AttachRequest),
    Detach(DetachRequest),
    ToggleMaximize,
    Minimize,
    StartDrag,
    StartResizeDrag(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WindowMode {
    is_tab_mode: bool,
    active_tab_labels: HashMap<String, String>,
    #[cfg(windows)]
    undecorated_resize: HashMap<String, isize>,
    #[cfg(not(windows))]
    host_signals: HashMap<String, u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TabState {
    tab_map: HashMap<String, Vec<Tab>>,
    closing: Vec<Tab>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ModeChangedArg {
    tab_mode: bool,
    webviews: Vec<WebviewTitle>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WebviewTitle {
    pub label: String,
    pub title: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ToggleTabModeRequest {
    tab_mode: bool,
    bounds: Option<Bounds>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AddTabRequest {
    opener: String,
    bounds: Bounds,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AttachRequest {
    pub from: String,
    pub to: String,
    pub attach_target: Option<String>,
    pub attach_before: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DetachRequest {
    pub label: String,
    pub offset_x: i32,
    pub offset_y: i32,
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
    host: String,
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

pub fn init(app: &tauri::AppHandle, host_name: &str) {
    let _ = HOST.set(host_name.to_string());

    app.manage(Mutex::new(TabState::default()));
    app.manage(Mutex::new(WindowMode::default()));

    #[cfg(windows)]
    {
        let mode = app.state::<Mutex<WindowMode>>();
        let mut mode = mode.lock().unwrap();
        platform_impl::prepare(app, &mut mode, host_name.to_string());
    }
}

pub fn update(app: &tauri::AppHandle, label: &str, title: &str, path: &str) {
    platform_impl::update(app, label, title, path);
}

pub fn remove(app: &tauri::AppHandle, label: &str) {
    platform_impl::remove(app, label);
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

    pub fn get_active_tab_label(&self, host_name: &str) -> &str {
        self.active_tab_labels.get(host_name).map(|s| s.as_str()).unwrap_or_default()
    }

    pub fn update_active_tab_label(&mut self, host_name: &str, label: &str) {
        self.active_tab_labels.insert(host_name.to_string(), label.to_string());
    }
}

pub struct ReparentResult {
    previous_host_name: String,
    tab: Tab,
}

impl TabState {
    pub fn all(&self) -> &HashMap<String, Vec<Tab>> {
        &self.tab_map
    }

    pub fn tabs(&self, key: &str) -> Option<&Vec<Tab>> {
        self.tab_map.get(key)
    }

    pub fn update(&mut self, key: &str, tabs: Vec<Tab>) {
        self.tab_map.insert(key.to_string(), tabs);
    }

    pub fn add(&mut self, key: &str, tab: Tab) {
        if let Some(tabs) = self.tab_map.get_mut(key) {
            tabs.push(tab);
        } else {
            self.update(key, vec![tab]);
        }
    }

    pub fn flatten(&self) -> Vec<&Tab> {
        self.tab_map.values().flatten().collect()
    }

    pub fn clear(&mut self) {
        self.tab_map.clear();
    }

    pub fn can_detach(&self, label: &str) -> bool {
        if let Some(tab) = self.find(label) {
            self.tab_map.get(&tab.host).unwrap_or(&Vec::new()).len() > 1
        } else {
            false
        }
    }

    pub fn find(&self, label: &str) -> Option<Tab> {
        for tabs in self.tab_map.values() {
            if let Some(tab) = tabs.iter().find(|tab| tab.label == label) {
                return Some(tab.clone());
            }
        }
        None
    }

    pub fn find_with_mut(&mut self, label: &str) -> Option<(&mut Tab, Vec<Tab>)> {
        for tabs in self.tab_map.values_mut() {
            let cloned = tabs.clone();
            if let Some(tab) = tabs.iter_mut().find(|tab| tab.label == label) {
                return Some((tab, cloned));
            }
        }
        None
    }

    pub fn position(&self, label: &str) -> Option<usize> {
        for tabs in self.tab_map.values() {
            if let Some(index) = tabs.iter().position(|tab| tab.label == label) {
                return Some(index);
            }
        }
        None
    }

    pub fn position_with(&self, label: &str) -> Option<(usize, Vec<Tab>)> {
        for tabs in self.tab_map.values() {
            let cloned = tabs.clone();
            if let Some(index) = tabs.iter().position(|tab| tab.label == label) {
                return Some((index, cloned));
            }
        }
        None
    }

    pub fn get(&self, key: &str, index: usize) -> Option<&Tab> {
        self.tab_map.get(key).unwrap().get(index)
    }

    pub fn remove(&mut self, key: &str) {
        self.tab_map.remove(key);
    }

    pub fn remove_tab(&mut self, label: &str) -> Option<Tab> {
        for tabs in self.tab_map.values_mut() {
            if let Some(index) = tabs.iter().position(|tab| tab.label == label) {
                let removed = tabs.remove(index);
                return Some(removed);
            }
        }

        None
    }

    pub fn get_host(&self, label: &str) -> String {
        for tabs in self.tab_map.values() {
            if let Some(tab) = tabs.iter().find(|tab| tab.label == label) {
                return tab.host.clone();
            }
        }

        HOST.get().unwrap().to_string()
    }

    pub fn find_host(&self, app: &tauri::AppHandle, label: &str) -> String {
        if !label.is_empty() {
            if let Some(tab) = self.find(label) {
                tab.host.to_string()
            } else {
                HOST.get().unwrap().to_string()
            }
        } else {
            if let Some(front) = app.webview_windows().iter().find(|(_, window)| window.is_visible().unwrap() && window.is_focused().unwrap()) {
                front.0.to_string()
            } else {
                HOST.get().unwrap().to_string()
            }
        }
    }

    pub fn close_all(&mut self, label: &str) {
        let host = self.get_host(label);
        if let Some(tabs) = self.tab_map.get(&host) {
            self.closing = tabs.clone();
        }
    }

    pub fn cancel_close_all(&mut self) {
        self.closing.clear();
    }

    pub fn reparent(&mut self, label: &str, new_host: &str) -> ReparentResult {
        let old = self.get_host(label);
        let old_tabs = self.tab_map.get_mut(&old).unwrap();
        let index = old_tabs.iter().position(|tab| tab.label == label).unwrap();
        let mut tab = old_tabs.remove(index);
        tab.host = new_host.to_string();
        self.add(new_host, tab.clone());
        ReparentResult {
            previous_host_name: old,
            tab,
        }
    }

    pub fn reparent_with_position(&mut self, label: &str, new_host: &str, target: Option<String>, attach_before: bool) -> ReparentResult {
        let old = self.get_host(label);
        let old_tabs = self.tab_map.get_mut(&old).unwrap();
        let index = old_tabs.iter().position(|tab| tab.label == label).unwrap();
        let mut tab = old_tabs.remove(index);
        tab.host = new_host.to_string();

        let tabs = self.tab_map.get_mut(new_host).unwrap();
        if let Some(target) = target {
            let mut index = tabs.iter().position(|tab| tab.label == target).unwrap();
            if !attach_before && index != tabs.len() - 1 {
                index += 1;
            }
            tabs.insert(index, tab.clone());
        } else {
            if attach_before {
                tabs.insert(0, tab.clone());
            } else {
                tabs.push(tab.clone());
            }
        }
        ReparentResult {
            previous_host_name: old,
            tab,
        }
    }
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

fn emit_filter(app: &tauri::AppHandle, event: TabEvent, tabs: &[Tab]) {
    for tab in tabs {
        let _ = app.emit_to(
            EventTarget::WebviewWindow {
                label: tab.label.to_string(),
            },
            "tab_event",
            event.clone(),
        );
    }
}

pub fn handle_request(window: &tauri::WebviewWindow, req: TabRequest) -> bool {
    match req {
        TabRequest::Add(request) => platform_impl::add(window, request),
        TabRequest::Attach(request) => platform_impl::attach(window.app_handle(), request),
        TabRequest::Detach(request) => platform_impl::detach(window.app_handle(), request),
        TabRequest::Cancel => platform_impl::cancel(window.app_handle()),
        TabRequest::Select(label) => platform_impl::select_tab(window.app_handle(), label),
        TabRequest::SelectNext => platform_impl::select(window, true),
        TabRequest::SelectPrevious => platform_impl::select(window, false),
        TabRequest::Reorder(tabs) => platform_impl::reorder_tab(window, tabs),
        TabRequest::CloseAll => platform_impl::close_all(window),
        TabRequest::Update(webview_title) => platform_impl::update(window.app_handle(), &webview_title.label, &webview_title.title, &webview_title.path),
        TabRequest::Close => platform_impl::close(window.app_handle(), window.label()),
        TabRequest::ToggleMaximize => platform_impl::toggle_maximize(window),
        TabRequest::Minimize => platform_impl::minimize(window),
        TabRequest::StartDrag => platform_impl::start_drag(window),
        TabRequest::StartResizeDrag(direction) => platform_impl::start_resize_dragging(window, direction),
        TabRequest::ToggleTabMode(request) => return platform_impl::toggle_tab_mode(window, request),
    }
    true
}
