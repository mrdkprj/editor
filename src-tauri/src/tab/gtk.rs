use crate::{
    helper::WindowLabels,
    tab::{emit, emit_to, Bounds, ModeChangedArg, Tab, TabEvent, TabState, Title, WebviewTitle, WindowMode, HOST},
};
use gtk::{
    ffi::GtkWidget,
    gdk::{
        traits::{DeviceExt, SeatExt},
        WindowEdge,
    },
    glib::{
        translate::{FromGlibPtrNone, ToGlibPtr},
        Cast,
    },
    traits::{BinExt, BoxExt, ContainerExt, GtkWindowExt, OverlayExt, WidgetExt},
};
use std::{collections::HashMap, sync::Mutex, time::Duration};
use tauri::{Manager, WebviewWindow};

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
            let tab = new_tab(app, window);
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

// added sent to current active child and then added child reordered
pub fn add(window: &tauri::WebviewWindow) {
    let app = window.app_handle();
    let label = window.label();
    let state = app.state::<Mutex<TabState>>();
    let mut state = state.lock().unwrap();

    let tab = if let Some(tab) = state.tabs.iter_mut().find(|tab| tab.label == label) {
        tab
    } else {
        let tab = new_tab(app, window);
        state.tabs.push(tab);
        state.tabs.last_mut().unwrap()
    };

    /* First send TabEvent::Added to the created window so that the tab looks active */
    let event = TabEvent::Added(Title {
        label: tab.label.clone(),
        title: tab.title.clone(),
        path: tab.path.clone(),
    });
    emit_to(app, event, &tab.label);

    tab.bounds = get_bounds(&app.get_webview_window(label).unwrap());
    let host = app.get_webview_window(HOST.get().unwrap()).unwrap();
    attach_to_tab(&host, tab);
    /* Delay switching for smooth rendering */
    bring_to_front_async(app, tab.clone());

    /* Unminimize */
    let app = app.clone();
    smol::spawn(async move {
        smol::Timer::after(Duration::from_millis(5)).await;
        let host = app.get_webview_window(HOST.get().unwrap()).unwrap();
        if host.is_minimized().unwrap() {
            host.unminimize().unwrap();
        }
        let _ = host.set_focus();
    })
    .detach();
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
    if let Some((index, tab)) = state.tabs.iter().enumerate().find(|(_, tab)| tab.label == label) {
        detach_from_tab(app, tab, false);

        if state.tabs.len() == 1 {
            /* If this is the last tab, hide the host */
            let host = app.get_webview_window(HOST.get().unwrap()).unwrap();
            let _ = host.hide();
        } else {
            /* Change active tab only instead of changing child to top-level window */
            let mode = app.state::<Mutex<WindowMode>>();
            let mut mode = mode.lock().unwrap();
            if mode.active_tab_label == label {
                let is_last = index == state.tabs.len() - 1;
                let tab = if is_last {
                    state.tabs.get(index - 1).unwrap()
                } else {
                    state.tabs.get(index).unwrap()
                };
                bring_to_front(app, &state.tabs, &mut mode, &tab.label);
            }
        }
    }
}

pub fn select_tab(app: &tauri::AppHandle, label: String) {
    let state = app.state::<Mutex<TabState>>();
    let state = state.lock().unwrap();
    let mode = app.state::<Mutex<WindowMode>>();
    let mut mode = mode.lock().unwrap();
    bring_to_front(app, &state.tabs, &mut mode, &label);
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
    let host = app.get_webview_window(HOST.get().unwrap()).unwrap();
    if host.is_maximized().unwrap_or_default() {
        let _ = host.unmaximize();
        emit(app, TabEvent::Unmaximized, None);
    } else {
        let _ = host.maximize();
        emit(app, TabEvent::Maximized, None);
    }
}

pub fn minimize(app: &tauri::AppHandle) {
    let _ = app.get_webview_window(HOST.get().unwrap()).unwrap().minimize();
}

fn enter_tab_mode(app: &tauri::AppHandle, tabs: &mut [Tab], mode: &mut WindowMode, activator: &str) {
    let host = app.get_webview_window(HOST.get().unwrap()).unwrap();

    change_to_overlay(&host);

    for tab in tabs.iter() {
        attach_to_tab(&host, tab);
    }

    bring_to_front(app, tabs, mode, activator);

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
    let host = app.get_webview_window(HOST.get().unwrap()).unwrap();

    mode.active_tab_label = String::new();

    for tab in tabs {
        detach_from_tab(app, tab, true);
    }

    restore_box(&host);

    let _ = host.hide();
}

pub(crate) fn remove(app: &tauri::AppHandle, label: &str) {
    let mode = app.state::<Mutex<WindowMode>>();
    let mode = mode.lock().unwrap();

    if !mode.tab_mode {
        return;
    }

    let state = app.state::<Mutex<TabState>>();
    let mut state = state.lock().unwrap();

    if let Some(index) = state.tabs.iter().position(|tab| tab.label == label) {
        let _ = state.tabs.remove(index);

        if !state.tabs.is_empty() {
            emit(app, TabEvent::Closed(label.to_string()), None);

            if mode.close_all {
                emit_to(app, TabEvent::Close(), &state.tabs.last().unwrap().label);
            }
        }
    }
}

pub(crate) fn start_drag(window: &tauri::WebviewWindow) {
    let app = window.app_handle();
    let mode = app.state::<Mutex<WindowMode>>();
    let mode = mode.lock().unwrap();
    if mode.tab_mode {
        let _ = app.get_webview_window(HOST.get().unwrap()).unwrap().start_dragging();
    } else {
        let _ = window.start_dragging();
    }
}

fn get_resize_edge(direction: &str) -> WindowEdge {
    match direction {
        "South" => WindowEdge::South,
        "SouthWest" => WindowEdge::SouthWest,
        "SouthEast" => WindowEdge::SouthEast,
        "West" => WindowEdge::West,
        "East" => WindowEdge::East,
        "North" => WindowEdge::North,
        "NorthWest" => WindowEdge::NorthWest,
        "NorthEast" => WindowEdge::NorthEast,
        _ => WindowEdge::North,
    }
}

pub(crate) fn start_resize_dragging(window: &tauri::WebviewWindow, direction: String) {
    let app = window.app_handle();
    let mode = app.state::<Mutex<WindowMode>>();
    let mode = mode.lock().unwrap();

    let window = if mode.tab_mode {
        app.get_webview_window(HOST.get().unwrap()).unwrap().gtk_window().unwrap()
    } else {
        window.gtk_window().unwrap()
    };
    if let Some(cursor) = window.display().default_seat().and_then(|seat| seat.pointer()) {
        let (_, x, y) = cursor.position();
        window.begin_resize_drag(get_resize_edge(&direction), 1, x, y, gtk::gdk::ffi::GDK_CURRENT_TIME as _);
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
    /* Must show so that menu can popup */
    child.gtk_window().unwrap().show();
    /*
        Use set_transient_for to prevent warning for context menu
        Couldn't map as window as popup because it doesn't have a parent
    */
    child.gtk_window().unwrap().set_transient_for(Some(&parent_window.gtk_window().unwrap()));

    /* Then Hide the original window */
    let _ = child.hide();
}

fn bring_to_front(app: &tauri::AppHandle, tabs: &[Tab], mode: &mut WindowMode, label: &str) {
    if mode.active_tab_label == label {
        return;
    }

    let host = app.get_webview_window(HOST.get().unwrap()).unwrap();
    let overlay = get_overlay(&host);

    if let Some(new) = tabs.iter().find(|s| s.label == label) {
        let webview = to_widget(new.window_handle);
        overlay.reorder_overlay(&webview, -1);
        emit_to(app, TabEvent::Activated, label);
        mode.active_tab_label = label.to_string();
    }
}

fn bring_to_front_async(app: &tauri::AppHandle, tab: Tab) {
    let app = app.clone();
    smol::spawn(async move {
        let state = app.state::<Mutex<TabState>>();
        if let Ok(state) = state.try_lock() {
            let mode = app.state::<Mutex<WindowMode>>();
            if let Ok(mut mode) = mode.try_lock() {
                bring_to_front(&app, &state.tabs, &mut mode, &tab.label);
            };
        };
        /* Send TabEvent::Added to others in delay to decrease flicker*/
        smol::Timer::after(Duration::from_millis(50)).await;
        emit(
            &app,
            TabEvent::Added(Title {
                label: tab.label.clone(),
                title: tab.title,
                path: tab.path,
            }),
            Some(&tab.label),
        );
    })
    .detach();
}

fn detach_from_tab(app: &tauri::AppHandle, removed: &Tab, show: bool) {
    if let Some(window) = app.get_webview_window(&removed.label) {
        let host = app.get_webview_window(HOST.get().unwrap()).unwrap();
        let overlay = get_overlay(&host);

        if overlay.children().len() > 1 {
            let webview = to_widget(removed.window_handle);
            overlay.remove(&webview);
            window.default_vbox().unwrap().pack_start(&webview, true, true, 0);

            if show {
                window.gtk_window().unwrap().hide();
                window.gtk_window().unwrap().set_transient_for(None::<&gtk::Window>);
                let child = app.clone();
                let label = removed.label.clone();
                gtk::glib::idle_add_local_once(move || {
                    let _ = child.get_webview_window(&label).unwrap().show();
                });
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

fn new_tab(app: &tauri::AppHandle, window: &tauri::WebviewWindow) -> Tab {
    let state = app.state::<Mutex<WindowLabels>>();
    let state = state.lock().unwrap();

    let (title, path) = if let Some(title) = state.labels.get(window.label()) {
        (title.title.clone(), title.path.clone())
    } else {
        (String::new(), String::new())
    };

    let window_handle = from_widget(window.default_vbox().unwrap().children().first().unwrap());

    Tab {
        window_handle,
        label: window.label().to_string(),
        title,
        path,
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
