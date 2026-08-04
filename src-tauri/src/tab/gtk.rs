use crate::{
    helper::WindowLabels,
    tab::{emit, emit_to, Bounds, ModeChangedArg, Tab, TabEvent, TabRequest, TabState, Title, WebviewTitle, WindowMode, TAB_WINDOW_LABEL},
};
use gtk::{
    ffi::GtkWidget,
    glib::{
        translate::{FromGlibPtrNone, ToGlibPtr},
        Cast,
    },
    traits::{BinExt, BoxExt, ContainerExt, OverlayExt, WidgetExt},
};
use std::{collections::HashMap, sync::Mutex, time::Duration};
use tauri::{Manager, WebviewWindow};

pub fn handle_request(window: &tauri::WebviewWindow, req: TabRequest) -> bool {
    match req {
        TabRequest::Add => add(window),
        TabRequest::Cancel => cancel(window.app_handle()),
        TabRequest::Select(label) => select_tab(window.app_handle(), label),
        TabRequest::Reorder(tabs) => reorder_tab(window, tabs),
        TabRequest::CloseAll => close_all(window.app_handle()),
        TabRequest::Update(webview_title) => update(window.app_handle(), &webview_title.label, &webview_title.title, &webview_title.path),
        TabRequest::Detach => detach(window.app_handle(), window.label()),
        TabRequest::ToggleMaximize => toggle_maximize(window.app_handle()),
        TabRequest::Minimize => minimize(window.app_handle()),
        TabRequest::ToggleTabMode(tab_mode) => return toggle_tab_mode(window, tab_mode),
    }
    true
}

pub fn toggle_tab_mode(window: &tauri::WebviewWindow, tab_mode: bool) -> bool {
    let app = window.app_handle();
    let mode = app.state::<Mutex<WindowMode>>();
    let mut mode = mode.lock().unwrap();
    let state = app.state::<Mutex<TabState>>();
    let mut state = state.lock().unwrap();

    let changed = mode.tab_mode != tab_mode;

    if mode.tab_mode != tab_mode {
        mode.tab_mode = tab_mode;
        if tab_mode {
            let tab = new_tab(app, from_widget(window.default_vbox().unwrap().children().first().unwrap()), window.label());
            if !state.tabs.contains(&tab) {
                state.tabs.push(tab);
            }
            enter_tab_mode(app, state.tabs.as_mut_slice(), &mut mode, window.label());
        } else {
            exit_tab_mode(app, &state.tabs, &mut mode);
        };
    }

    let titles: Vec<WebviewTitle> = if mode.tab_mode {
        state
            .tabs
            .iter()
            .map(|a| WebviewTitle {
                label: a.label.clone(),
                title: a.title.clone(),
                path: a.path.clone(),
            })
            .collect()
    } else {
        Vec::new()
    };

    emit(
        app,
        TabEvent::ModeChanged(ModeChangedArg {
            tab_mode: mode.tab_mode,
            tabs: titles,
        }),
        None,
    );

    changed
}

pub fn add(window: &tauri::WebviewWindow) {
    let app = window.app_handle();
    let label = window.label();
    let state = app.state::<Mutex<TabState>>();
    let mut state = state.lock().unwrap();

    let tab = if let Some(tab) = state.tabs.iter_mut().find(|tab| tab.label == label) {
        tab
    } else {
        let tab = new_tab(app, from_widget(window.default_vbox().unwrap().children().first().unwrap()), window.label());
        state.tabs.push(tab);
        state.tabs.last_mut().unwrap()
    };
    tab.bounds = get_bounds(&app.get_webview_window(label).unwrap());
    let host = app.get_webview_window(TAB_WINDOW_LABEL).unwrap();
    attach_to_tab(&host, tab);
    /* Delay switching for smooth rendering */
    bring_to_front_async(app, tab.clone());
}

pub fn update(app: &tauri::AppHandle, label: &str, title: &str, path: &str) {
    let state = app.state::<Mutex<TabState>>();
    let mut state = state.lock().unwrap();

    if let Some(index) = state.tabs.iter().position(|tab| tab.label == label) {
        let tab = state.tabs.get_mut(index).unwrap();
        tab.title = title.to_string();
        tab.path = path.to_string();
        emit(
            app,
            TabEvent::TitleChanged(Title {
                label: label.to_string(),
                title: title.to_string(),
                path: path.to_string(),
            }),
            Some(label),
        );
    }
}

pub fn detach(app: &tauri::AppHandle, label: &str) {
    let state = app.state::<Mutex<TabState>>();
    let state = state.lock().unwrap();
    if let Some(tab) = state.tabs.iter().find(|tab| tab.label == label) {
        detach_from_tab(app, tab, false);
    }
}

pub fn select_tab(app: &tauri::AppHandle, label: String) {
    let state = app.state::<Mutex<TabState>>();
    let state = state.lock().unwrap();
    let mode = app.state::<Mutex<WindowMode>>();
    let mut mode = mode.lock().unwrap();
    bring_to_front(app, &state.tabs, &mut mode, label);
}

pub fn reorder_tab(window: &tauri::WebviewWindow, tabs: Vec<WebviewTitle>) {
    let app = window.app_handle();
    let state = app.state::<Mutex<TabState>>();
    let mut state = state.lock().unwrap();
    let mut new = Vec::new();
    let mp: HashMap<String, Tab> = state.tabs.iter().map(|t| (t.label.clone(), t.clone())).collect();
    for tab in &tabs {
        if let Some(a) = mp.get(&tab.label) {
            new.push(a.clone());
        }
    }
    state.tabs = new;
    emit(app, TabEvent::Reordered(tabs), Some(window.label()));
}

pub fn close_all(app: &tauri::AppHandle) {
    let state = app.state::<Mutex<TabState>>();
    let state = state.lock().unwrap();
    let mode = app.state::<Mutex<WindowMode>>();
    let mut mode = mode.lock().unwrap();
    mode.close_all = true;
    let target = &state.tabs.last().unwrap().label;
    emit_to(app, TabEvent::Close(), target);
}

pub fn cancel(app: &tauri::AppHandle) {
    let mode = app.state::<Mutex<WindowMode>>();
    let mut mode = mode.lock().unwrap();
    mode.close_all = false;
}

pub fn toggle_maximize(app: &tauri::AppHandle) {
    let host = app.get_webview_window(TAB_WINDOW_LABEL).unwrap();
    if host.is_maximized().unwrap_or_default() {
        let _ = host.unmaximize();
        emit(app, TabEvent::Unmaximized, None);
    } else {
        let _ = host.maximize();
        emit(app, TabEvent::Maximized, None);
    }
}

pub fn minimize(app: &tauri::AppHandle) {
    let _ = app.get_webview_window(TAB_WINDOW_LABEL).unwrap().minimize();
}

fn enter_tab_mode(app: &tauri::AppHandle, tabs: &mut [Tab], mode: &mut WindowMode, activator: &str) {
    let host = app.get_webview_window(TAB_WINDOW_LABEL).unwrap();

    change_to_overlay(&host);

    for tab in tabs.iter() {
        attach_to_tab(&host, tab);
    }

    bring_to_front(app, tabs, mode, activator.to_string());

    let _ = host.show();
}

fn change_to_overlay(host: &tauri::WebviewWindow) {
    /*
        Change Window's child from Box to Overlay
        Tauri expects this hierarchy
        Window > gtk Container > Webview
        So add webivew instead of Box directly to Overlay
    */
    let host_window = host.gtk_window().unwrap();
    let host_box: gtk::Box = host_window.child().unwrap().dynamic_cast().unwrap();
    let overlay = gtk::Overlay::new();
    let children = host_box.children();
    let webview = children.first().unwrap();
    host_box.remove(webview);
    overlay.add_overlay(webview);
    overlay.reorder_overlay(webview, 0);
    host_window.remove(&host_box);
    host_window.add(&overlay);
}

fn restore_box(host: &tauri::WebviewWindow) {
    let host_window = host.gtk_window().unwrap();
    let overlay = get_overlay(host);
    let children = overlay.children();
    let webview = children.first().unwrap();
    overlay.remove(webview);
    let host_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
    host_box.pack_start(webview, true, true, 0);
    host_window.remove(&overlay);
    host_window.add(&host_box);
}

fn exit_tab_mode(app: &tauri::AppHandle, tabs: &[Tab], mode: &mut WindowMode) {
    let host = app.get_webview_window(TAB_WINDOW_LABEL).unwrap();

    mode.active_tab_label = String::new();

    for tab in tabs {
        detach_from_tab(app, tab, true);
    }

    restore_box(&host);

    let _ = host.hide();
}

pub(crate) fn remove(app: &tauri::AppHandle, label: &str) {
    let state = app.state::<Mutex<TabState>>();
    let mut state = state.lock().unwrap();
    let mode = app.state::<Mutex<WindowMode>>();
    let mut mode = mode.lock().unwrap();

    if !mode.tab_mode {
        return;
    }

    if let Some(index) = state.tabs.iter().position(|tab| tab.label == label) {
        let _ = state.tabs.remove(index);
        if state.tabs.is_empty() {
            let host = app.get_webview_window(TAB_WINDOW_LABEL).unwrap();
            let _ = host.hide();
        } else {
            let is_last = index == state.tabs.len();

            if mode.active_tab_label == label && !state.tabs.is_empty() {
                let tab = if is_last {
                    state.tabs.get(index - 1).unwrap()
                } else {
                    state.tabs.get(index).unwrap()
                };
                bring_to_front(app, &state.tabs, &mut mode, tab.label.clone());
            }
            emit(app, TabEvent::Closed(label.to_string()), None);
            if mode.close_all {
                emit_to(app, TabEvent::Close(), &state.tabs.last().unwrap().label);
            }
        }
    }
}

fn attach_to_tab(parent_window: &WebviewWindow, tab: &Tab) {
    let vbox = get_overlay(parent_window);
    let child = parent_window.get_webview_window(&tab.label).unwrap();
    let child_vbox = child.default_vbox().unwrap();
    let webview = to_widget(tab.window_handle);
    child_vbox.remove(&webview);
    vbox.add_overlay(&webview);
    vbox.reorder_overlay(&webview, 0);
    webview.show();

    /* Hide the original window */
    let _ = child.hide();
}

fn bring_to_front(app: &tauri::AppHandle, tabs: &[Tab], mode: &mut WindowMode, label: String) {
    if mode.active_tab_label == label {
        return;
    }

    let host = app.get_webview_window(TAB_WINDOW_LABEL).unwrap();
    let overlay = get_overlay(&host);

    if let Some(new) = tabs.iter().find(|s| s.label == label) {
        overlay.reorder_overlay(&to_widget(new.window_handle), -1);
        emit_to(app, TabEvent::Activated, &label);
        mode.active_tab_label = label;
    }
}

fn bring_to_front_async(app: &tauri::AppHandle, tab: Tab) {
    let app = app.clone();
    smol::spawn(async move {
        smol::Timer::after(Duration::from_millis(50)).await;
        let state = app.state::<Mutex<TabState>>();
        let state = state.lock().unwrap();
        let mode = app.state::<Mutex<WindowMode>>();
        let mut mode = mode.lock().unwrap();
        bring_to_front(&app, &state.tabs, &mut mode, tab.label.clone());
        emit(
            &app,
            TabEvent::Added(Title {
                label: tab.label,
                title: tab.title,
                path: tab.path,
            }),
            None,
        );
    })
    .detach();
}

fn detach_from_tab(app: &tauri::AppHandle, removed: &Tab, show: bool) {
    if let Some(window) = app.get_webview_window(&removed.label) {
        let host = app.get_webview_window(TAB_WINDOW_LABEL).unwrap();
        let overlay = get_overlay(&host);

        if overlay.children().len() > 1 {
            let webview = to_widget(removed.window_handle);
            overlay.remove(&webview);
            window.default_vbox().unwrap().pack_start(&webview, true, true, 0);
            if show {
                let _ = window.show();
            }
        }
    }
}

fn get_bounds(window: &tauri::WebviewWindow) -> Bounds {
    let pos = window.outer_position().unwrap();
    let size = window.outer_size().unwrap();
    Bounds {
        width: size.width,
        height: size.height,
        x: pos.x,
        y: pos.y,
    }
}

fn new_tab(app: &tauri::AppHandle, window_handle: isize, label: &str) -> Tab {
    let state = app.state::<Mutex<WindowLabels>>();
    let state = state.lock().unwrap();

    let (title, path) = if let Some(title) = state.labels.get(label) {
        (title.title.clone(), title.path.clone())
    } else {
        (String::new(), String::new())
    };

    Tab {
        window_handle,
        label: label.to_string(),
        title,
        path,
        inset: super::WindowInset {
            x: 0,
            y: 0,
        },
        style: 0,
        parent: None,
        owner: None,
        active: false,
        bounds: Bounds::default(),
    }
}

fn get_overlay(window: &tauri::WebviewWindow) -> gtk::Overlay {
    let vbox: gtk::Overlay = window.gtk_window().unwrap().child().unwrap().dynamic_cast().unwrap();
    vbox
}

fn from_widget(gbox: &gtk::Widget) -> isize {
    let ptr: *mut GtkWidget = gbox.to_glib_none().0;
    ptr as isize
}

fn to_widget(prt: isize) -> gtk::Widget {
    let window: gtk::Widget = unsafe { gtk::Widget::from_glib_none(prt as *mut GtkWidget) };
    window
}
