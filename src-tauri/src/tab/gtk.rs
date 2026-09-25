use crate::{
    helper::WindowLabels,
    tab::{emit, emit_filter, emit_to, AddTabRequest, AttachRequest, Bounds, DetachRequest, ModeChangedArg, Tab, TabEvent, TabState, ToggleTabModeRequest, WebviewTitle, WindowMode, HOST},
};
use gtk::{
    ffi::GtkWidget,
    gdk::{
        traits::{DeviceExt, SeatExt},
        WindowEdge, WindowState,
    },
    glib::{
        translate::{FromGlib, FromGlibPtrNone, ToGlibPtr},
        Cast, ObjectExt, SignalHandlerId,
    },
    traits::{BinExt, BoxExt, ContainerExt, GtkWindowExt, OverlayExt, WidgetExt},
};
use std::{collections::HashMap, sync::Mutex, time::Duration};
use tauri::{Manager, PhysicalSize, WebviewWindow};

pub fn toggle_tab_mode(window: &tauri::WebviewWindow, request: ToggleTabModeRequest) -> bool {
    let app = window.app_handle();
    let mode = app.state::<Mutex<WindowMode>>();
    let mut mode = mode.lock().unwrap();
    let state = app.state::<Mutex<TabState>>();
    let mut state = state.lock().unwrap();

    let changed = mode.can_toggle_mode(request.tab_mode);
    if changed {
        if request.tab_mode {
            enter_tab_mode(app, &mut state, &mut mode, window.label(), request);
        } else {
            exit_tab_mode(app, &mut state, &mut mode);
        };
    }

    let webviews: Vec<WebviewTitle> = if changed && mode.is_tab_mode() {
        /* If changed, there's only one host */
        state
            .flatten()
            .iter()
            .map(|tab| WebviewTitle {
                label: tab.label.clone(),
                title: tab.title.clone(),
                path: tab.path.clone(),
            })
            .collect()
    } else {
        Vec::new()
    };

    emit(
        app,
        TabEvent::ModeChanged(ModeChangedArg {
            tab_mode: mode.is_tab_mode(),
            webviews,
        }),
        None,
    );

    changed
}

pub fn add(window: &tauri::WebviewWindow, request: AddTabRequest) {
    let app = window.app_handle();
    let state = app.state::<Mutex<TabState>>();
    let mut state = state.lock().unwrap();

    /*
        If this is called immediately after enter_tab_mode, the window is already tabbed.
        In this case, just bring the tab to front without emitting added event.
    */
    if let Some(tab) = state.find(window.label()) {
        bring_to_front_async(app, tab, None);
        return;
    }

    let label = window.label();

    let host_name = state.find_host(app, &request.opener);
    /* Before attach and show this tab, send current tab data to the window */
    let tabs = state.tabs(&host_name).unwrap();
    let titles: Vec<WebviewTitle> = tabs
        .iter()
        .map(|tab| WebviewTitle {
            label: tab.label.clone(),
            title: tab.title.clone(),
            path: tab.path.clone(),
        })
        .collect();
    emit_to(app, TabEvent::Attached(titles), label);

    let mut tab = new_tab(app, window, Some(&host_name));
    tab.bounds = request.bounds;
    state.add(&host_name, tab.clone());

    let host = app.get_webview_window(&host_name).unwrap();
    attach_to_tab(&host, &tab);
    /* Delay switching for smooth rendering */
    bring_to_front_async(app, tab, Some(state.tabs(&host_name).unwrap().clone()));

    /* Unminimize */
    let app = app.clone();
    smol::spawn(async move {
        smol::Timer::after(Duration::from_millis(5)).await;
        let host = app.get_webview_window(&host_name).unwrap();
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

    if let Some((tab, tabs)) = state.find_with_mut(label) {
        tab.title = title.to_string();
        tab.path = path.to_string();
        emit_filter(
            app,
            TabEvent::TitleChanged(WebviewTitle {
                label: label.to_string(),
                title: title.to_string(),
                path: path.to_string(),
            }),
            &tabs,
        );
    }
}

pub fn attach(app: &tauri::AppHandle, request: AttachRequest) {
    let mode = app.state::<Mutex<WindowMode>>();
    let mut mode = mode.lock().unwrap();
    let state = app.state::<Mutex<TabState>>();
    let mut state = state.lock().unwrap();

    let old_tab = state.find(&request.from).unwrap();
    /* Change active tab of the detached tabs */
    shift_active_tab(app, &state, &mut mode, &old_tab.host, &old_tab.label);

    let new_host_name = state.get_host(&request.to);
    let result = state.reparent_with_position(&request.from, &new_host_name, request.attach_target, request.attach_before);

    let detached_tabs = state.tabs(&result.previous_host_name).unwrap();
    if detached_tabs.is_empty() {
        state.remove(&result.previous_host_name);
        /* If no tabs remain, destroy the host except default one */
        if hide_host(app, &result.previous_host_name) {
            state.remove(&result.previous_host_name);
            mode.remove(&result.previous_host_name);
            let old_host = app.get_webview_window(&result.previous_host_name).unwrap();
            let _ = old_host.destroy();
        }
    } else {
        /* Notify this tab is detached */
        emit_filter(app, TabEvent::Closed(result.tab.label.clone()), state.tabs(&result.previous_host_name).unwrap());
    }

    /* Reset tab data on frontend */
    let tab = result.tab;
    let tabs = state.tabs(&tab.host).unwrap();
    let webviews = tabs
        .iter()
        .map(|tab| WebviewTitle {
            label: tab.label.clone(),
            title: tab.title.clone(),
            path: tab.path.clone(),
        })
        .collect();
    emit_filter(
        app,
        TabEvent::ModeChanged(ModeChangedArg {
            tab_mode: true,
            webviews,
        }),
        tabs,
    );

    let old_host = app.get_webview_window(&result.previous_host_name).unwrap();
    let new_host = app.get_webview_window(&tab.host).unwrap();
    reparent(&old_host, &new_host, &tab);

    bring_to_front_async(app, tab, None);
}

pub fn detach(app: &tauri::AppHandle, request: DetachRequest) {
    let app = app.clone();

    let state = app.state::<Mutex<TabState>>();
    let mut state = state.lock().unwrap();

    if !state.can_detach(&request.label) {
        return;
    }

    let mode = app.state::<Mutex<WindowMode>>();
    let mut mode = mode.lock().unwrap();

    let old_tab = state.find(&request.label).unwrap();
    /* Change active tab of the detached tabs */
    shift_active_tab(&app, &state, &mut mode, &old_tab.host, &old_tab.label);

    let new_host_name = crate::helper::create_new_host_window(&app);
    let new_host = app.get_webview_window(&new_host_name).unwrap();
    change_to_overlay(&new_host, &mut mode);

    let result = state.reparent(&request.label, &new_host_name);
    let old_host = app.get_webview_window(&result.previous_host_name).unwrap();
    /* Make the old host top-most */
    old_host.set_focus().unwrap();

    /* Notify this tab is detached */
    emit_filter(&app, TabEvent::Closed(result.tab.label.clone()), state.tabs(&result.previous_host_name).unwrap());

    /* Reset tab data on frontend */
    let tab = result.tab;
    let tabs = state.tabs(&tab.host).unwrap();
    let webviews = tabs
        .iter()
        .map(|tab| WebviewTitle {
            label: tab.label.clone(),
            title: tab.title.clone(),
            path: tab.path.clone(),
        })
        .collect();
    emit_filter(
        &app,
        TabEvent::ModeChanged(ModeChangedArg {
            tab_mode: true,
            webviews,
        }),
        tabs,
    );

    reparent(&old_host, &new_host, &tab);
    new_host.set_size(PhysicalSize::new(tab.bounds.width, tab.bounds.height)).unwrap();
    new_host.show().unwrap();

    bring_to_front(&app, &state, &mut mode, &tab.label);
}

pub fn close(app: &tauri::AppHandle, label: &str) {
    let state = app.state::<Mutex<TabState>>();
    let state = state.lock().unwrap();

    if let Some(tab) = state.find(label) {
        detach_from_tab(app, &tab, false);

        let mode = app.state::<Mutex<WindowMode>>();
        let mut mode = mode.lock().unwrap();

        let tabs = state.tabs(&tab.host).unwrap();
        if tabs.len() == 1 {
            /* If this is the last tab, hide the host */
            hide_host(app, &tab.host);
        } else {
            /* Change active tab only instead of changing child to top-level window */
            shift_active_tab(app, &state, &mut mode, &tab.host, &tab.label);
        }
    }
}

pub fn select_tab(app: &tauri::AppHandle, label: String) {
    let state = app.state::<Mutex<TabState>>();
    let state = state.lock().unwrap();
    let mode = app.state::<Mutex<WindowMode>>();
    let mut mode = mode.lock().unwrap();
    bring_to_front(app, &state, &mut mode, &label);
}

pub fn select(window: &tauri::WebviewWindow, next: bool) {
    let app = window.app_handle();
    let state = app.state::<Mutex<TabState>>();
    let state = state.lock().unwrap();
    let mode = app.state::<Mutex<WindowMode>>();
    let mut mode = mode.lock().unwrap();

    if let Some((index, tabs)) = state.position_with(window.label()) {
        if tabs.len() <= 1 {
            return;
        }

        if next {
            if index == tabs.len() - 1 {
                bring_to_front(app, &state, &mut mode, &tabs[0].label);
            } else {
                bring_to_front(app, &state, &mut mode, &tabs[index + 1].label);
            }
        } else {
            if index == 0 {
                bring_to_front(app, &state, &mut mode, &tabs[tabs.len() - 1].label);
            } else {
                bring_to_front(app, &state, &mut mode, &tabs[index - 1].label);
            }
        }
    }
}

pub fn reorder_tab(window: &tauri::WebviewWindow, reordered_tabs: Vec<WebviewTitle>) {
    let app = window.app_handle();
    let state = app.state::<Mutex<TabState>>();
    let mut state = state.lock().unwrap();
    let mut new_tabs = Vec::new();
    let sample = &reordered_tabs.first().unwrap().label;
    let host_name = state.get_host(sample);
    let mp: HashMap<String, Tab> = state.tabs(&host_name).unwrap().iter().map(|tab| (tab.label.clone(), tab.clone())).collect();
    for reordered in &reordered_tabs {
        if let Some(tab) = mp.get(&reordered.label) {
            new_tabs.push(tab.clone());
        }
    }
    state.update(&host_name, new_tabs.clone());
    emit_filter(app, TabEvent::Reordered(reordered_tabs), &new_tabs);
}

pub fn close_all(window: &tauri::WebviewWindow) {
    let app = window.app_handle();
    let state = app.state::<Mutex<TabState>>();
    let mut state = state.lock().unwrap();
    state.close_all(window.label());
    if let Some(tab) = state.closing.pop() {
        emit_to(app, TabEvent::Close, &tab.label);
    }
}

pub fn cancel(app: &tauri::AppHandle) {
    let state = app.state::<Mutex<TabState>>();
    let mut state = state.lock().unwrap();
    state.cancel_close_all();
}

pub fn toggle_maximize(window: &tauri::WebviewWindow) {
    /* Prevent state lock blocking */
    let host_name = {
        let app = window.app_handle();
        let state = app.state::<Mutex<TabState>>();
        let state = state.lock().unwrap();
        state.find_host(app, window.label())
    };
    let host = window.get_webview_window(&host_name).unwrap();
    if host.is_maximized().unwrap_or_default() {
        let _ = host.unmaximize();
    } else {
        let _ = host.maximize();
    }
}

pub fn minimize(window: &tauri::WebviewWindow) {
    let app = window.app_handle();
    let state = app.state::<Mutex<TabState>>();
    let state = state.lock().unwrap();
    if let Some(tab) = state.find(window.label()) {
        let host = window.get_webview_window(&tab.host).unwrap();
        let _ = host.minimize();
    }
}

fn shift_active_tab(app: &tauri::AppHandle, state: &TabState, mode: &mut WindowMode, host_name: &str, label: &str) {
    if let Some(index) = state.position(label) {
        if mode.get_active_tab_label(host_name).unwrap_or_default() == label {
            let tabs = state.tabs(host_name).unwrap();
            if tabs.len() > 1 {
                let tab = if index == 0 {
                    state.get(host_name, index + 1).unwrap()
                } else {
                    state.get(host_name, index - 1).unwrap()
                };
                bring_to_front(app, state, mode, &tab.label);
            }
        }
    }
}

fn hide_host(app: &tauri::AppHandle, host_name: &str) -> bool {
    let host = app.get_webview_window(host_name).unwrap();
    let _ = host.hide();
    host_name != HOST.get().unwrap()
}

fn enter_tab_mode(app: &tauri::AppHandle, state: &mut TabState, mode: &mut WindowMode, activator: &str, request: ToggleTabModeRequest) {
    mode.enter();

    let host_name = HOST.get().unwrap();
    let host = app.get_webview_window(host_name).unwrap();

    change_to_overlay(&host, mode);

    let mut tabs: Vec<Tab> = Vec::new();

    for (label, window) in app.webview_windows() {
        if &label == host_name {
            continue;
        }
        let mut tab = new_tab(app, &window, Some(host_name));
        let bounds = if window.is_visible().unwrap_or_default() {
            get_bounds(&window)
        } else if let Some(bounds) = &request.bounds {
            bounds.clone()
        } else {
            Bounds::default()
        };
        tab.bounds = bounds;
        attach_to_tab(&host, &tab);
        tabs.push(tab);
    }

    /* Must insert before bring to front */
    state.update(host_name, tabs);

    bring_to_front(app, state, mode, activator);

    /* On Wayland, can't get size of the hidden window */
    let size = if let Some(bounds) = request.bounds {
        PhysicalSize::new(bounds.width, bounds.height)
    } else {
        let activator_window = app.get_webview_window(activator).unwrap();
        activator_window.outer_size().unwrap()
    };

    host.set_size(size).unwrap();
    host.unmaximize().unwrap();
    host.show().unwrap();
}

fn change_to_overlay(host: &tauri::WebviewWindow, mode: &mut WindowMode) {
    /*
        Change Window's child from Box to Overlay
        Tauri expects this hierarchy
        Window > gtk Container > Webview
        So add webivew directly to Overlay instead of Box
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
    let app = host.app_handle().clone();
    let host_name = host.label().to_string();
    let signal = host_window.connect_window_state_event(move |_, e| {
        let state = app.state::<Mutex<TabState>>();
        if let Ok(state) = state.try_lock() {
            if let Some(tabs) = state.tabs(&host_name) {
                if e.new_window_state().contains(WindowState::MAXIMIZED) {
                    emit_filter(&app, TabEvent::Maximized, tabs);
                }

                if e.changed_mask().contains(WindowState::MAXIMIZED) && !e.new_window_state().contains(WindowState::MAXIMIZED) {
                    emit_filter(&app, TabEvent::Unmaximized, tabs);
                }
            }
        };
        gtk::glib::Propagation::Proceed
    });
    if let Some(old) = mode.host_signals.insert(host.label().to_string(), unsafe { signal.as_raw() }) {
        host_window.disconnect(unsafe { SignalHandlerId::from_glib(old) });
    }
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

fn reparent(old: &tauri::WebviewWindow, new: &tauri::WebviewWindow, tab: &Tab) {
    let old_overlay = get_overlay(old);
    let webview = to_widget(tab.window_handle);
    old_overlay.remove(&webview);
    let new_overlay = get_overlay(new);
    new_overlay.add_overlay(&webview);
}

fn exit_tab_mode(app: &tauri::AppHandle, state: &mut TabState, mode: &mut WindowMode) {
    mode.exit();

    for (host_name, tabs) in state.all() {
        for tab in tabs.iter() {
            detach_from_tab(app, tab, true);
        }

        let host = app.get_webview_window(host_name).unwrap();
        restore_box(&host);

        let _ = host.hide();
        if host_name != HOST.get().unwrap() {
            let _ = host.destroy();
        }
    }
    state.clear();
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

fn bring_to_front(app: &tauri::AppHandle, state: &TabState, mode: &mut WindowMode, label: &str) {
    if let Some(tab) = state.find(label) {
        if mode.get_active_tab_label(&tab.host).unwrap_or_default() == label {
            return;
        }

        let host = app.get_webview_window(&tab.host).unwrap();
        let overlay = get_overlay(&host);
        let webview = to_widget(tab.window_handle);
        overlay.reorder_overlay(&webview, -1);
        emit_to(app, TabEvent::Activated, label);
        mode.update_active_tab_label(&tab.host, label);
    }
}

fn bring_to_front_async(app: &tauri::AppHandle, tab: Tab, emit_targets: Option<Vec<Tab>>) {
    let app = app.clone();
    smol::spawn(async move {
        smol::Timer::after(Duration::from_millis(50)).await;
        let state = app.state::<Mutex<TabState>>();
        if let Ok(state) = state.try_lock() {
            let mode = app.state::<Mutex<WindowMode>>();
            if let Ok(mut mode) = mode.try_lock() {
                bring_to_front(&app, &state, &mut mode, &tab.label);
            };

            if let Some(tabs) = emit_targets {
                emit_filter(
                    &app,
                    TabEvent::Added(WebviewTitle {
                        label: tab.label.clone(),
                        title: tab.title.clone(),
                        path: tab.path.clone(),
                    }),
                    &tabs,
                );
            }
        };
    })
    .detach();
}

pub(crate) fn remove(app: &tauri::AppHandle, label: &str) {
    let mode = app.state::<Mutex<WindowMode>>();
    let mode = mode.lock().unwrap();

    if !mode.is_tab_mode() {
        return;
    }

    let state = app.state::<Mutex<TabState>>();
    let mut state = state.lock().unwrap();

    if let Some(removed) = state.remove_tab(label) {
        let tabs = state.tabs(&removed.host).unwrap();

        if !tabs.is_empty() {
            emit_filter(app, TabEvent::Closed(label.to_string()), tabs);

            if let Some(tab) = state.closing.pop() {
                emit_to(app, TabEvent::Close, &tab.label);
            }
        }
    }
}

pub(crate) fn start_drag(window: &tauri::WebviewWindow) {
    let app = window.app_handle();
    let mode = app.state::<Mutex<WindowMode>>();
    let mode = mode.lock().unwrap();
    if mode.is_tab_mode() {
        let state = app.state::<Mutex<TabState>>();
        let state = state.lock().unwrap();
        let host_name = state.get_host(window.label());
        let _ = app.get_webview_window(&host_name).unwrap().start_dragging();
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

    let window = if mode.is_tab_mode() {
        let state = app.state::<Mutex<TabState>>();
        let state = state.lock().unwrap();
        let host_name = state.get_host(window.label());
        app.get_webview_window(&host_name).unwrap().gtk_window().unwrap()
    } else {
        window.gtk_window().unwrap()
    };
    if let Some(cursor) = window.display().default_seat().and_then(|seat| seat.pointer()) {
        let (_, x, y) = cursor.position();
        window.begin_resize_drag(get_resize_edge(&direction), 1, x, y, gtk::gdk::ffi::GDK_CURRENT_TIME as _);
    }
}

fn detach_from_tab(app: &tauri::AppHandle, removed: &Tab, show: bool) {
    if let Some(window) = app.get_webview_window(&removed.label) {
        let host = app.get_webview_window(&removed.host).unwrap();
        let overlay = get_overlay(&host);

        if overlay.children().len() > 1 {
            let webview = to_widget(removed.window_handle);
            overlay.remove(&webview);
            window.default_vbox().unwrap().pack_start(&webview, true, true, 0);

            window.gtk_window().unwrap().hide();
            window.gtk_window().unwrap().set_transient_for(None::<&gtk::Window>);
            let app = app.clone();
            let label = removed.label.clone();
            let size = if show {
                PhysicalSize::new(removed.bounds.width, removed.bounds.height)
            } else {
                host.outer_size().unwrap_or_default()
            };

            gtk::glib::idle_add_local_once(move || {
                let window = app.get_webview_window(&label).unwrap();
                if !show {
                    /*
                        Can't set size of the hidden window.
                        So make the window transparent and show it before setting size.
                    */
                    window.gtk_window().unwrap().set_opacity(0.0);
                }
                let _ = window.show();
                window.set_size(size).unwrap();
            });
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

fn new_tab(app: &tauri::AppHandle, window: &tauri::WebviewWindow, host_name: Option<&str>) -> Tab {
    let host = host_name.unwrap_or(HOST.get().unwrap()).to_string();

    let state = app.state::<Mutex<WindowLabels>>();
    let state = state.lock().unwrap();

    let (title, path) = if let Some(title) = state.labels.get(window.label()) {
        (title.title.clone(), title.path.clone())
    } else {
        (String::new(), String::new())
    };

    let window_handle = from_widget(window.default_vbox().unwrap().children().first().unwrap());

    Tab {
        host,
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
