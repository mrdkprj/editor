use serde::{Deserialize, Serialize};

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
    DragResize(String),
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
    pub resizing: bool,
    pub propagated: bool,
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
    active: bool,
    inset: WindowInset,
    style: isize,
    parent: Option<isize>,
    owner: Option<isize>,
    bounds: Bounds,
}

impl PartialEq for Tab {
    fn eq(&self, other: &Self) -> bool {
        self.label.eq(&other.label)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct WindowInset {
    x: i32,
    y: i32,
}
