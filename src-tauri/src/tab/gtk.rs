#![allow(unused_imports)]
#[cfg(target_os = "windows")]
use crate::tab::WindowInset;
use crate::{
    helper::WindowLabels,
    tab::{Bounds, ModeChangedArg, Tab, TabEvent, TabRequest, TabState, Title, WebviewTitle, WindowMode},
};
#[cfg(target_os = "linux")]
use gtk::{
    glib::Cast,
    traits::{BoxExt, ContainerExt, OverlayExt, WidgetExt},
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Mutex, time::Duration};
use tauri::{Emitter, EventTarget, Manager, WebviewWindow};
use windows::Win32::UI::Shell::{DefSubclassProc, SetWindowSubclass};
#[cfg(target_os = "windows")]
use windows::Win32::{
    Foundation::{HWND, POINT, RECT},
    Graphics::Gdi::ClientToScreen,
    UI::{Shell::RemoveWindowSubclass, WindowsAndMessaging::*},
};

const OFF_SCREEN: i32 = -30000;

#[derive(Debug, PartialEq)]
pub(crate) enum WindowType {
    Top,
    Child,
    Owned,
}

pub fn init(app: &tauri::AppHandle) {
    app.manage(Mutex::new(TabState::default()));
    app.manage(Mutex::new(WindowMode::default()));

    let cloned = app.clone();
    app.get_webview_window("Main").unwrap().on_window_event(move |e| match e {
        tauri::WindowEvent::Focused(focused) => {
            let mode = cloned.state::<Mutex<WindowMode>>();
            let mode = mode.lock().unwrap();
            if mode.tab_mode && *focused {
                emit_to(&cloned, TabEvent::Activated, &mode.active_tab_label);
            }
        }
        tauri::WindowEvent::Resized(size) => {
            let mode = cloned.state::<Mutex<WindowMode>>();
            if let Ok(mode) = mode.try_lock() {
                if mode.tab_mode {
                    let state = cloned.state::<Mutex<TabState>>();
                    let state = state.lock().unwrap();
                    on_resizing(&state.tabs, size.width as i32, size.height as i32);
                }
            };
        }
        _ => {}
    });
}

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
            let tab = new_tab(app, vtoi(window.hwnd().unwrap()), window.label());
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
        let tab = new_tab(app, vtoi(window.hwnd().unwrap()), window.label());
        state.tabs.push(tab);
        state.tabs.last_mut().unwrap()
    };
    tab.bounds = get_bounds(&app.get_webview_window(label).unwrap());
    let host = app.get_webview_window("Main").unwrap();
    let size = host.inner_size().unwrap();
    before_attach(window);
    attach_to_tab(&host, tab, size.width as _, size.height as _);
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
        detach_from_tab(tab, false);
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
    let host = app.get_webview_window("Main").unwrap();
    if host.is_maximized().unwrap_or_default() {
        let _ = host.unmaximize();
        emit(app, TabEvent::Unmaximized, None);
    } else {
        let _ = host.maximize();
        emit(app, TabEvent::Maximized, None);
    }
    after_toggle(app);
}

fn after_toggle(app: &tauri::AppHandle) {
    let mode = app.state::<Mutex<WindowMode>>();
    if let Ok(mode) = mode.try_lock() {
        on_resized(mode.window_handle);
    };
}

pub fn minimize(app: &tauri::AppHandle) {
    let _ = app.get_webview_window("Main").unwrap().minimize();
}

fn enter_tab_mode(app: &tauri::AppHandle, tabs: &mut [Tab], mode: &mut WindowMode, activator: &str) {
    let host = app.get_webview_window("Main").unwrap();

    #[cfg(target_os = "windows")]
    {
        // unsafe {
        //     let current_style = GetWindowLongPtrW(host.hwnd().unwrap(), GWL_STYLE) as u32;
        //     if (current_style & WS_CLIPCHILDREN.0) == 0 {
        //         SetWindowLongPtrW(host.hwnd().unwrap(), GWL_STYLE, (current_style | WS_CLIPCHILDREN.0) as isize);
        //     }
        // }

        /* Must show parent first. Otherwise, extra top margin shows */
        let _ = unsafe { SetWindowPos(host.hwnd().unwrap(), None, OFF_SCREEN, OFF_SCREEN, 0, 0, SWP_NOSIZE | SWP_NOZORDER | SWP_FRAMECHANGED) };
        host.unmaximize().unwrap();
        host.with_webview(|w| unsafe { w.controller().SetIsVisible(false).unwrap() }).unwrap();
        host.show().unwrap();

        let activator_window = app.get_webview_window(activator).unwrap();
        let size = activator_window.outer_size().unwrap();
        let pos = activator_window.outer_position().unwrap();

        host.set_size(size).unwrap();

        for tab in tabs.iter_mut() {
            let child = app.get_webview_window(&tab.label).unwrap();
            let bounds = get_bounds(&child);
            tab.bounds = bounds;
            before_attach(&child);
            attach_to_tab(&host, tab, size.width as _, size.height as _);
        }

        bring_to_front(app, tabs, mode, activator.to_string());

        host.set_position(pos).unwrap();

        let _ = unsafe { SetWindowSubclass(host.hwnd().unwrap(), Some(proc), 200, Box::into_raw(Box::new(app.clone())) as usize) };
    }
    #[cfg(target_os = "linux")]
    {
        let host = app.get_webview_window("Main").unwrap();
        let vbox = host.default_vbox().unwrap();
        let host_children = vbox.children();
        let widget = host_children.get(1).unwrap();
        let overlay: gtk::Overlay = widget.clone().downcast().unwrap();

        for child_label in labels {
            let child = app.get_webview_window(&child_label).unwrap();
            let child_vbox = child.default_vbox().unwrap();
            let children = child_vbox.children();
            if let Some(tab_webview) = children.first() {
                child_vbox.remove(tab_webview);
                tab_webview.set_widget_name(&child_label);
                tab_webview.set_hexpand(true);
                tab_webview.set_vexpand(true);
                tab_webview.hide();

                overlay.add_overlay(tab_webview);
                /* Place the webview at the bottom of the overlay stack */
                overlay.reorder_overlay(tab_webview, 0);
            }
        }
    }
}

//   host.as_ref().window().start_resize_dragging(direction);
fn exit_tab_mode(app: &tauri::AppHandle, tabs: &[Tab], mode: &mut WindowMode) {
    #[cfg(target_os = "windows")]
    {
        mode.active_tab_label = String::new();

        let host = app.get_webview_window("Main").unwrap();

        for tab in tabs.iter() {
            detach_from_tab(tab, true);
            after_detach(tab.window_handle);
        }

        remove_subclass(host.hwnd().unwrap());
        host.hide().unwrap();
    }
    #[cfg(target_os = "linux")]
    {
        let host = app.get_webview_window("Main").unwrap();
        let vbox = host.default_vbox().unwrap();
        let vbox_children = vbox.children();
        if vbox_children.len() > 1 {
            let widget = vbox_children.get(1).unwrap();
            let overlay: gtk::Overlay = widget.clone().downcast().unwrap();

            for child in overlay.children() {
                if child.widget_name() == label {
                    overlay.remove(&child);
                    let child_vbox = window.default_vbox().unwrap();
                    child_vbox.pack_start(&child, true, true, 0);
                }
            }
        }
    }
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
            let host = app.get_webview_window("Main").unwrap();
            remove_subclass(host.hwnd().unwrap());
            let _ = unsafe { SetWindowPos(host.hwnd().unwrap(), None, OFF_SCREEN, OFF_SCREEN, 0, 0, SWP_NOSIZE) };
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

fn before_attach(window: &tauri::WebviewWindow) {
    /* On Windows, window must be shown before attach. Otherwise, focus can be moved to the parent */
    let _ = unsafe { SetWindowPos(window.hwnd().unwrap(), None, OFF_SCREEN, OFF_SCREEN, 0, 0, SWP_NOSIZE | SWP_SHOWWINDOW | SWP_NOACTIVATE) };
    let _ = unsafe { SetWindowSubclass(window.hwnd().unwrap(), Some(child_proc), 300, Box::into_raw(Box::new(window.app_handle().clone())) as usize) };
}

fn after_detach(window_handle: isize) {
    let _ = unsafe { RemoveWindowSubclass(to_hwnd(window_handle), Some(child_proc), 300) };
}

fn attach_to_tab(parent_window: &WebviewWindow, tab: &Tab, width: i32, height: i32) {
    let parent = parent_window.hwnd().unwrap();
    let child = to_hwnd(tab.window_handle);

    let mut style = unsafe { GetWindowLongPtrW(child, GWL_STYLE) } as u32;
    style &= !(WS_POPUP.0);
    // style &= !(WS_SIZEBOX.0);
    style |= WS_CLIPSIBLINGS.0;
    style |= WS_CHILD.0;
    unsafe { SetWindowLongPtrW(child, GWL_STYLE, style as isize) };
    let ex_style = unsafe { GetWindowLongPtrW(child, GWL_EXSTYLE) } as u32;
    unsafe { SetWindowLongPtrW(child, GWL_EXSTYLE, (ex_style | WS_EX_LAYERED.0) as isize) };

    unsafe { SetParent(child, Some(parent)).unwrap() };

    let _ = unsafe { SetWindowPos(child, Some(HWND_BOTTOM), -tab.inset.x, 0, width + tab.inset.x * 2, height + tab.inset.y * 2, SWP_FRAMECHANGED | SWP_NOACTIVATE) };
}

fn bring_to_front(_app: &tauri::AppHandle, tabs: &[Tab], mode: &mut WindowMode, label: String) {
    #[cfg(target_os = "windows")]
    {
        if mode.active_tab_label == label {
            return;
        }

        if let Some(new) = tabs.iter().find(|tab| tab.label == label) {
            let _ = unsafe { SetWindowPos(to_hwnd(new.window_handle), Some(HWND_TOP), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE) };

            emit_to(_app, TabEvent::Activated, &label);
            unsafe {
                use windows::Win32::UI::Input::KeyboardAndMouse::SetFocus;
                let _ = SetFocus(Some(to_hwnd(new.window_handle)));
            }
            mode.active_tab_label = label;
            mode.window_handle = new.window_handle;
        }
    }
    #[cfg(target_os = "linux")]
    {
        let host = app.get_webview_window("Main").unwrap();
        let vbox = host.default_vbox().unwrap();
        let vbox_children = vbox.children();
        if vbox_children.len() > 1 {
            let widget = vbox_children.get(1).unwrap();
            let overlay: gtk::Overlay = widget.clone().downcast().unwrap();

            for child in overlay.children() {
                if child.widget_name() == label {
                    overlay.reorder_overlay(&child, -1);
                }
            }
        }
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

fn detach_from_tab(removed: &Tab, show: bool) {
    unsafe { SetWindowLongPtrW(to_hwnd(removed.window_handle), GWL_STYLE, removed.style) };

    if let Some(parent) = removed.parent {
        unsafe { SetParent(to_hwnd(removed.window_handle), Some(to_hwnd(parent))).unwrap() };
    } else {
        unsafe { SetParent(to_hwnd(removed.window_handle), None).unwrap() };
    }

    if let Some(owner) = removed.owner {
        unsafe { SetWindowLongPtrW(to_hwnd(removed.window_handle), GWLP_HWNDPARENT, owner) };
    }

    if show {
        let _ =
            unsafe { SetWindowPos(to_hwnd(removed.window_handle), None, removed.bounds.x, removed.bounds.y, removed.bounds.width as _, removed.bounds.height as _, SWP_FRAMECHANGED | SWP_SHOWWINDOW) };
        // let _ = unsafe { ShowWindow(to_hwnd(removed.window_handle), SW_SHOW) };
    } else {
        let _ = unsafe { SetWindowPos(to_hwnd(removed.window_handle), None, OFF_SCREEN, OFF_SCREEN, removed.bounds.width as _, removed.bounds.height as _, SWP_FRAMECHANGED | SWP_SHOWWINDOW) };
        let _ = unsafe { SetWindowPos(to_hwnd(removed.window_handle), None, removed.bounds.x, removed.bounds.y, 0, 0, SWP_NOSIZE | SWP_HIDEWINDOW) };
        // let _ = unsafe { ShowWindow(to_hwnd(removed.window_handle), SW_HIDE) };
    }
}

fn on_resizing(tabs: &[Tab], width: i32, height: i32) {
    for tab in tabs {
        let _ = unsafe {
            SetWindowPos(to_hwnd(tab.window_handle), None, 0, 0, width + tab.inset.x * 2, height + tab.inset.y * 2, SWP_NOMOVE | SWP_NOZORDER | SWP_NOCOPYBITS | SWP_NOACTIVATE | SWP_NOSENDCHANGING)
        };
    }
}

fn on_resized(window_handle: isize) {
    let _ = unsafe { SetWindowPos(to_hwnd(window_handle), Some(HWND_TOP), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE) };
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
    let hwnd = HWND(window_handle as _);
    let inset = get_exact_hwnd_insets(hwnd);
    let window_type = get_window_type(hwnd);
    let parent = if window_type == WindowType::Child {
        Some(vtoi(unsafe { GetParent(hwnd).unwrap() }))
    } else {
        None
    };

    let owner = if window_type == WindowType::Owned {
        Some(vtoi(unsafe { GetWindow(hwnd, GW_OWNER).unwrap() }))
    } else {
        None
    };

    Tab {
        window_handle,
        label: label.to_string(),
        title,
        path,
        inset,
        style: unsafe { GetWindowLongPtrW(hwnd, GWL_STYLE) },
        parent,
        owner,
        active: false,
        bounds: Bounds::default(),
    }
}
