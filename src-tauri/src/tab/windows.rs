use crate::{
    helper::WindowLabels,
    tab::{
        emit, emit_filter, emit_to, AttachRequest, Bounds, DetachRequest, ModeChangedArg, Tab,
        TabEvent::{self},
        TabState, WebviewTitle, WindowInset, WindowMode, HOST,
    },
};
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU16, Ordering::Relaxed},
        Mutex,
    },
    time::Duration,
};
use tauri::{Manager, PhysicalSize, WebviewWindow};
use windows::{
    core::{Free, PCWSTR},
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, POINT, POINTS, RECT, WPARAM},
        Graphics::Gdi::{ClientToScreen, CreateRectRgn, GetWindowRgn, SetWindowRgn, RGN_ERROR},
        UI::{
            Input::KeyboardAndMouse::{ReleaseCapture, SendInput, SetFocus, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, VK_ESCAPE},
            Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass},
            WindowsAndMessaging::*,
        },
    },
};

const OFF_SCREEN: i32 = -30000;
const TOP_RESIZE_BORDER_SIZE: i32 = 1;
const PARENT_SUBCLASS_ID: usize = WM_USER as usize + 1;
const RESIZE_SUBCLASS_ID: usize = WM_USER as usize + 2;
const CHILD_SUBCLASS_ID: usize = WM_USER as usize + 3;

static SUBCLASS_ID: AtomicU16 = AtomicU16::new(0);

#[derive(Debug, PartialEq)]
pub(crate) enum WindowType {
    Top,
    Child,
    Owned,
}

struct ResizeData {
    app: tauri::AppHandle,
    host_name: String,
}

pub fn prepare(app: &tauri::AppHandle, host_name: String) -> isize {
    let host = app.get_webview_window(&host_name).unwrap();
    let mode = app.state::<Mutex<WindowMode>>();
    let mut mode = mode.lock().unwrap();

    /*
        To override the region set by Tauri's undecorated_resizing, we need to install a subclass of the resize window.
        In case Tauri change will change its class name, find the child window that has window region.
        If no window is found or multiple windows are found, cause panic not to proceed.
    */
    let mut children = Vec::new();
    let mut start_child = HWND::default();
    while let Ok(child) = unsafe { FindWindowExW(Some(host.hwnd().unwrap()), Some(start_child), PCWSTR::null(), PCWSTR::null()) } {
        let mut region = unsafe { CreateRectRgn(0, 0, 0, 0) };
        if unsafe { GetWindowRgn(child, region) } != RGN_ERROR {
            children.push(child.0 as isize);
        }
        unsafe { region.free() };
        start_child = child;
    }

    if children.is_empty() || children.len() > 1 {
        panic!("Can't find undecorated_resize window");
    }

    let undecorated_resize = *children.first().unwrap();
    mode.update_undecorated_resize(&host_name, undecorated_resize);
    undecorated_resize
}

pub fn toggle_tab_mode(window: &tauri::WebviewWindow, tab_mode: bool) -> bool {
    let app = window.app_handle();

    let mode = app.state::<Mutex<WindowMode>>();
    let mut mode = mode.lock().unwrap();
    let state = app.state::<Mutex<TabState>>();
    let mut state = state.lock().unwrap();

    let changed = mode.can_toggle_mode(tab_mode);
    if changed {
        if tab_mode {
            enter_tab_mode(app, &mut state, &mut mode, window.label());
        } else {
            exit_tab_mode(app, &mut state, &mut mode);
        };
    }

    let webviews: Vec<WebviewTitle> = if changed && mode.is_tab_mode() {
        /* If changed, there's only one host which is default one */
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

pub fn add(window: &tauri::WebviewWindow, activator: String) {
    let app = window.app_handle();
    let state = app.state::<Mutex<TabState>>();
    let mut state = state.lock().unwrap();

    let label = window.label();
    let hwnd = window.hwnd().unwrap();

    let host_name = state.find_host(app, &activator);
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

    let mut tab = new_tab(app, Some(&host_name), vtoi(hwnd), window.label());
    tab.bounds = get_bounds(&app.get_webview_window(label).unwrap());

    state.add(&host_name, tab.clone());

    let host = app.get_webview_window(&host_name).unwrap();
    let size = host.inner_size().unwrap();

    before_attach(window);
    attach_to_tab(&host, &tab, size.width as _, size.height as _);
    /* Delay switching for smooth rendering */
    bring_to_front_async(app, tab);

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

pub fn attach(app: &tauri::AppHandle, req: AttachRequest) {
    let mode = app.state::<Mutex<WindowMode>>();
    let mut mode = mode.lock().unwrap();
    let state = app.state::<Mutex<TabState>>();
    let mut state = state.lock().unwrap();

    let new_host = state.get_host(&req.to);
    let (old_host, tab) = state.reparent(&req.from, &new_host);
    println!("attach");
    /* Notify this tab is detached */
    emit_filter(app, TabEvent::Closed(tab.label.clone()), state.tabs(&old_host).unwrap());

    /* Notify this tab is added */
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

    let host = app.get_webview_window(&tab.host).unwrap();
    let host_hwnd = host.hwnd().unwrap();
    let size = host.outer_size().unwrap();
    let child = app.get_webview_window(&tab.label).unwrap();
    let child = child.hwnd().unwrap();
    reparent(child, host_hwnd, &tab, size);

    bring_to_front(app, &state, &mut mode, &tab.label);
}

unsafe fn cancel_drag() {
    /* Create the Key Down event */
    let press = INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VK_ESCAPE,
                wScan: 0,
                dwFlags: Default::default(),
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };

    /* Create the Key Up event */
    let release = INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VK_ESCAPE,
                wScan: 0,
                dwFlags: KEYEVENTF_KEYUP,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };
    SendInput(&[press, release], size_of::<INPUT>() as i32);
}

pub fn detach(app: &tauri::AppHandle, req: DetachRequest) {
    let app = app.clone();

    /* Must cancel dragging */
    unsafe { cancel_drag() };

    /* On Windows, window creation must be on a different thread and on the main thread */
    smol::spawn(async move {
        let app = app.clone();
        app.clone()
            .run_on_main_thread(move || {
                let state = app.state::<Mutex<TabState>>();
                let mut state = state.lock().unwrap();

                let host_name = crate::helper::create_new_host_window(&app);
                println!("new host:{:?}", host_name);
                let (old_host, tab) = state.reparent(&req.label, &host_name);

                /* Notify this tab is detached */
                emit_filter(&app, TabEvent::Closed(tab.label.clone()), state.tabs(&old_host).unwrap());

                /* Notify this tab is added */
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

                let host = app.get_webview_window(&host_name).unwrap();
                let host_hwnd = host.hwnd().unwrap();

                let undecorated_resize = prepare(&app, host_name.clone());
                install_subclass(&app, host_hwnd, &host_name, undecorated_resize);

                let activator_window = app.get_webview_window(&req.label).unwrap();
                let size = activator_window.outer_size().unwrap();
                let mut pos = activator_window.outer_position().unwrap();

                let child = activator_window.hwnd().unwrap();
                reparent(child, host_hwnd, &tab, size);
                let _ = unsafe { SetFocus(Some(child)) };

                host.set_size(size).unwrap();
                let mut lppoint = POINT::default();
                let _ = unsafe { GetCursorPos(&mut lppoint) };
                pos.x = lppoint.x - 30;
                pos.y = lppoint.y - 30;
                host.set_position(pos).unwrap();
                host.unmaximize().unwrap();
                host.show().unwrap();

                /* Start dragging the new host window */
                let points = POINTS {
                    x: lppoint.x as _,
                    y: lppoint.y as _,
                };
                let _ = unsafe { PostMessageW(Some(host_hwnd), WM_NCLBUTTONDOWN, WPARAM(HTCAPTION as _), LPARAM(&points as *const _ as _)) };
            })
            .unwrap();
    })
    .detach();
}

fn reparent(child: HWND, parent: HWND, tab: &Tab, size: PhysicalSize<u32>) {
    unsafe { SetParent(child, Some(parent)).unwrap() };
    let _ = unsafe {
        SetWindowPos(
            child,
            Some(HWND_TOP),
            -tab.inset.x,
            -TOP_RESIZE_BORDER_SIZE,
            size.width as i32 + tab.inset.x * 2,
            size.height as i32 + TOP_RESIZE_BORDER_SIZE + tab.inset.y * 2,
            SWP_FRAMECHANGED | SWP_NOACTIVATE,
        )
    };
}

pub fn close(app: &tauri::AppHandle, label: &str) {
    let state = app.state::<Mutex<TabState>>();
    let state = state.lock().unwrap();

    if let Some((index, tab)) = state.enumerate(label) {
        let position = app.get_webview_window(&tab.host).unwrap().outer_position().unwrap();
        /* Set parent's position */
        let _ = unsafe { SetWindowPos(to_hwnd(tab.window_handle), Some(HWND_BOTTOM), position.x, position.y, tab.bounds.width as _, tab.bounds.height as _, SWP_HIDEWINDOW) };
        detach_from_tab(tab, false);

        let mode = app.state::<Mutex<WindowMode>>();
        let mut mode = mode.lock().unwrap();

        let same_host_tabs = state.tabs(&tab.host).unwrap();
        if same_host_tabs.len() == 1 {
            /* If this is the last tab, hide the host */
            let host = app.get_webview_window(&tab.host).unwrap();
            let _ = unsafe { SetWindowPos(host.hwnd().unwrap(), None, OFF_SCREEN, OFF_SCREEN, 0, 0, SWP_NOSIZE) };
            uninstall_subclass(host.hwnd().unwrap(), mode.get_undecorated_resize(&tab.host));
            let _ = host.hide();
        } else {
            /* Change active tab only instead of changing child to top-level window */
            if mode.get_active_tab_label(&tab.host) == label {
                let is_last = index == same_host_tabs.len() - 1;
                let tab = if is_last {
                    state.get(&tab.host, index - 1).unwrap()
                } else {
                    state.get(&tab.host, index).unwrap()
                };
                bring_to_front(app, &state, &mut mode, &tab.label);
            }
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
    state.insert(&host_name, new_tabs.clone());
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
    let app = window.app_handle();
    let state = app.state::<Mutex<TabState>>();
    let state = state.lock().unwrap();
    if let Some((tab, tabs)) = state.find_with(window.label()) {
        let host = window.get_webview_window(&tab.host).unwrap();
        if host.is_maximized().unwrap_or_default() {
            let _ = host.unmaximize();
            emit_filter(app, TabEvent::Unmaximized, tabs);
        } else {
            let _ = host.maximize();
            emit_filter(app, TabEvent::Maximized, tabs);
        }
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

fn install_subclass(app: &tauri::AppHandle, host: HWND, host_name: &str, undecorated_resize: isize) {
    unsafe {
        let current_style = GetWindowLongPtrW(host, GWL_STYLE) as u32;
        if (current_style & WS_CLIPCHILDREN.0) == 0 {
            SetWindowLongPtrW(host, GWL_STYLE, (current_style | WS_CLIPCHILDREN.0) as isize);
        }
    }

    let parent_subclass_id = PARENT_SUBCLASS_ID + SUBCLASS_ID.fetch_add(1, Relaxed) as usize;
    let resize_subclass_id = RESIZE_SUBCLASS_ID + SUBCLASS_ID.fetch_add(1, Relaxed) as usize;
    let resize_data = ResizeData {
        app: app.clone(),
        host_name: host_name.to_string(),
    };
    let _ = unsafe { SetWindowSubclass(host, Some(subclass_parent), parent_subclass_id, Box::into_raw(Box::new(resize_data)) as _) };
    let _ = unsafe { SetWindowSubclass(to_hwnd(undecorated_resize), Some(resize_subclass), resize_subclass_id, host.0 as _) };
}

fn uninstall_subclass(host: HWND, undecorated_resize: isize) {
    let _ = unsafe { RemoveWindowSubclass(to_hwnd(undecorated_resize), Some(resize_subclass), RESIZE_SUBCLASS_ID) };
    let _ = unsafe { RemoveWindowSubclass(host, Some(subclass_parent), PARENT_SUBCLASS_ID) };
}

fn enter_tab_mode(app: &tauri::AppHandle, state: &mut TabState, mode: &mut WindowMode, activator: &str) {
    mode.enter();

    let host_name = HOST.get().unwrap();
    let host = app.get_webview_window(host_name).unwrap();
    let undecorated_resize = mode.get_undecorated_resize(host_name);
    install_subclass(app, host.hwnd().unwrap(), host_name, undecorated_resize);

    let activator_window = app.get_webview_window(activator).unwrap();
    let size = activator_window.outer_size().unwrap();
    let pos = activator_window.outer_position().unwrap();

    let mut tabs: Vec<Tab> = Vec::new();

    for (label, window) in app.webview_windows() {
        if &label == host_name {
            continue;
        }

        let mut tab = new_tab(app, None, vtoi(window.hwnd().unwrap()), window.label());
        let bounds = get_bounds(&window);
        tab.bounds = bounds;
        before_attach(&window);
        attach_to_tab(&host, &tab, size.width as _, size.height as _);
        tabs.push(tab);
    }

    /* Must insert before bring to front */
    state.insert(host_name, tabs);

    bring_to_front(app, state, mode, activator);

    host.set_size(size).unwrap();
    host.set_position(pos).unwrap();
    host.unmaximize().unwrap();
    host.show().unwrap();
}

fn exit_tab_mode(app: &tauri::AppHandle, state: &mut TabState, mode: &mut WindowMode) {
    mode.exit();

    for (host_name, tabs) in state.all() {
        let host = app.get_webview_window(host_name).unwrap();
        let undecorated_resize = mode.get_undecorated_resize(host_name);
        uninstall_subclass(host.hwnd().unwrap(), undecorated_resize);
        let _ = host.hide();

        for tab in tabs.iter() {
            detach_from_tab(tab, true);
        }

        if host_name != HOST.get().unwrap() {
            let _ = host.destroy();
        }
    }

    state.clear();
}

fn before_attach(window: &tauri::WebviewWindow) {
    /* Without this, focus is strange */
    let _ = unsafe { SetWindowPos(window.hwnd().unwrap(), None, OFF_SCREEN, OFF_SCREEN, 0, 0, SWP_NOSIZE | SWP_SHOWWINDOW | SWP_NOACTIVATE) };
}

fn attach_to_tab(parent_window: &WebviewWindow, tab: &Tab, width: i32, height: i32) {
    let parent = parent_window.hwnd().unwrap();
    let child = to_hwnd(tab.window_handle);

    let mut style = unsafe { GetWindowLongPtrW(child, GWL_STYLE) } as u32;
    style &= !(WS_POPUP.0);
    style |= WS_CLIPSIBLINGS.0;
    style |= WS_CHILD.0;
    unsafe { SetWindowLongPtrW(child, GWL_STYLE, style as isize) };
    let ex_style = unsafe { GetWindowLongPtrW(child, GWL_EXSTYLE) } as u32;
    unsafe { SetWindowLongPtrW(child, GWL_EXSTYLE, (ex_style | WS_EX_LAYERED.0) as isize) };

    unsafe { SetParent(child, Some(parent)).unwrap() };

    let _ = unsafe {
        SetWindowPos(child, Some(HWND_BOTTOM), -tab.inset.x, -TOP_RESIZE_BORDER_SIZE, width + tab.inset.x * 2, height + TOP_RESIZE_BORDER_SIZE + tab.inset.y * 2, SWP_FRAMECHANGED | SWP_NOACTIVATE)
    };

    let _ = unsafe { SetWindowSubclass(child, Some(child_proc), CHILD_SUBCLASS_ID, Box::into_raw(Box::new(parent_window.app_handle().clone())) as usize) };
}

fn bring_to_front(app: &tauri::AppHandle, state: &TabState, mode: &mut WindowMode, label: &str) {
    if let Some(tab) = state.find(label) {
        if mode.get_active_tab_label(&tab.host) == label {
            return;
        }

        let _ = unsafe { SetWindowPos(to_hwnd(tab.window_handle), Some(HWND_TOP), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE) };
        emit_to(app, TabEvent::Activated, label);
        let _ = unsafe { SetFocus(Some(to_hwnd(tab.window_handle))) };

        mode.update_active_tab_label(&tab.host, label);
    }
}

fn bring_to_front_async(app: &tauri::AppHandle, tab: Tab) {
    let app = app.clone();
    smol::spawn(async move {
        smol::Timer::after(Duration::from_millis(50)).await;
        let state = app.state::<Mutex<TabState>>();
        if let Ok(state) = state.try_lock() {
            let mode = app.state::<Mutex<WindowMode>>();
            if let Ok(mut mode) = mode.try_lock() {
                bring_to_front(&app, &state, &mut mode, &tab.label);
                let tabs = state.tabs(&tab.host).unwrap();
                emit_filter(
                    &app,
                    TabEvent::Added(WebviewTitle {
                        label: tab.label.clone(),
                        title: tab.title.clone(),
                        path: tab.path.clone(),
                    }),
                    tabs,
                );
            };
        };
    })
    .detach();
}

pub(crate) fn remove(app: &tauri::AppHandle, label: &str) {
    println!("remove");
    let mode = app.state::<Mutex<WindowMode>>();
    let mode = mode.lock().unwrap();

    if !mode.is_tab_mode() {
        return;
    }

    let state = app.state::<Mutex<TabState>>();
    let mut state = state.lock().unwrap();

    if let Some(removed) = state.remove_by_label(label) {
        let tabs = state.tabs(&removed.host).unwrap();

        if !tabs.is_empty() {
            emit_filter(app, TabEvent::Closed(label.to_string()), tabs);

            if let Some(tab) = state.closing.pop() {
                emit_to(app, TabEvent::Close, &tab.label);
            }
        }
    }
}

#[allow(dead_code, unused_variables)]
pub(crate) fn start_drag(window: &tauri::WebviewWindow) {
    /* css "-webkit-app-region: drag" does on Windows */
}

fn get_resize_edge(direction: &str) -> u32 {
    match direction {
        "South" => WMSZ_BOTTOM,
        "SouthWest" => WMSZ_BOTTOMLEFT,
        "SouthEast" => WMSZ_BOTTOMRIGHT,
        "West" => WMSZ_LEFT,
        "East" => WMSZ_RIGHT,
        "North" => WMSZ_TOP,
        "NorthWest" => WMSZ_TOPLEFT,
        "NorthEast" => WMSZ_TOPRIGHT,
        _ => WMSZ_TOP,
    }
}

pub(crate) fn start_resize_dragging(window: &tauri::WebviewWindow, direction: String) {
    let edge = get_resize_edge(&direction);
    if let Ok(hwnd) = window.hwnd() {
        let points = {
            let mut pos = POINT::default();
            let _ = unsafe { GetCursorPos(&mut pos) };
            pos
        };
        let points = POINTS {
            x: points.x as i16,
            y: points.y as i16,
        };

        drag_resize_window(hwnd, WPARAM(edge as usize), LPARAM(&points as *const _ as _));
    }
}

fn drag_resize_window(hwnd: HWND, wparam: WPARAM, lparam: LPARAM) {
    let _ = unsafe { ReleaseCapture() };
    let _ = unsafe { PostMessageW(Some(hwnd), WM_NCLBUTTONDOWN, wparam, lparam) };
}

fn detach_from_tab(removed: &Tab, show: bool) {
    /* Restore style only when showing window. Otherwise, closing tabs causes flicker */
    if show {
        unsafe { SetWindowLongPtrW(to_hwnd(removed.window_handle), GWL_STYLE, removed.style) };
    }

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
    }

    let _ = unsafe { RemoveWindowSubclass(to_hwnd(removed.window_handle), Some(child_proc), CHILD_SUBCLASS_ID) };
}

unsafe extern "system" fn subclass_parent(child: HWND, umsg: u32, wparam: WPARAM, lparam: LPARAM, _uidsubclass: usize, dwrefdata: usize) -> LRESULT {
    if umsg == WM_WINDOWPOSCHANGED {
        let mut rect = RECT::default();

        if GetClientRect(child, &mut rect).is_ok() {
            let width = rect.right - rect.left;
            let height = rect.bottom - rect.top;
            let item_data_ptr = dwrefdata as *const ResizeData;
            let data = &*item_data_ptr;
            let state = data.app.state::<Mutex<TabState>>();
            if let Ok(state) = state.try_lock() {
                if let Some(tabs) = state.tabs(&data.host_name) {
                    on_resized(tabs, width, height);
                }
            };
        }
    }

    DefSubclassProc(child, umsg, wparam, lparam)
}

unsafe extern "system" fn resize_subclass(child: HWND, umsg: u32, wparam: WPARAM, lparam: LPARAM, _uidsubclass: usize, dwrefdata: usize) -> LRESULT {
    if umsg == WM_WINDOWPOSCHANGED {
        let parent = to_hwnd(dwrefdata as _);

        if !is_maximized(parent).unwrap_or(false) {
            let mut rect = RECT::default();

            if GetClientRect(parent, &mut rect).is_ok() {
                let width = rect.right - rect.left;
                let height = rect.bottom - rect.top;
                let _ = SetWindowPos(child, Some(HWND_TOP), 0, 0, width, height, SWP_ASYNCWINDOWPOS | SWP_NOACTIVATE | SWP_NOOWNERZORDER | SWP_NOMOVE | SWP_NOSIZE);
                /* Height must be 0 to remove extra top region */
                /* hrgn1 must be mutable to call .free() later */
                let mut hrgn1 = CreateRectRgn(0, 0, width, 0);

                if SetWindowRgn(child, Some(hrgn1), true) == 0 {
                    hrgn1.free();
                }
            }
        }
    }

    DefSubclassProc(child, umsg, wparam, lparam)
}

fn is_maximized(window: HWND) -> windows::core::Result<bool> {
    let mut placement = WINDOWPLACEMENT {
        length: std::mem::size_of::<WINDOWPLACEMENT>() as u32,
        ..WINDOWPLACEMENT::default()
    };
    unsafe { GetWindowPlacement(window, &mut placement)? };
    Ok(placement.showCmd == SW_MAXIMIZE.0 as u32)
}

unsafe extern "system" fn child_proc(hwnd: HWND, umsg: u32, wparam: WPARAM, lparam: LPARAM, _uidsubclass: usize, _dwrefdata: usize) -> LRESULT {
    if umsg == WM_NCLBUTTONDOWN {
        let hit_test = wparam.0 as u32;

        let is_resize_edge = matches!(hit_test, HTTOP | HTBOTTOM | HTLEFT | HTRIGHT | HTTOPLEFT | HTTOPRIGHT | HTBOTTOMLEFT | HTBOTTOMRIGHT);

        if is_resize_edge {
            if let Ok(parent_hwnd) = GetParent(hwnd) {
                drag_resize_window(parent_hwnd, wparam, lparam);
                /*
                   Return 0 so DefSubclassProc is NOT called for the child.
                   This prevents the child from entering WM_ENTERSIZEMOVE entirely.
                */
                return LRESULT(0);
            }
        }
    }

    DefSubclassProc(hwnd, umsg, wparam, lparam)
}

fn on_resized(tabs: &[Tab], width: i32, height: i32) {
    for tab in tabs {
        let _ = unsafe {
            SetWindowPos(
                to_hwnd(tab.window_handle),
                None,
                0,
                0,
                width + tab.inset.x * 2,
                height + TOP_RESIZE_BORDER_SIZE + tab.inset.y * 2,
                SWP_NOMOVE | SWP_NOZORDER | SWP_NOCOPYBITS | SWP_NOACTIVATE | SWP_NOSENDCHANGING | SWP_ASYNCWINDOWPOS,
            )
        };
    }
}

fn to_hwnd(ptr: isize) -> HWND {
    HWND(ptr as *mut std::ffi::c_void)
}

fn vtoi(hwnd: HWND) -> isize {
    hwnd.0 as isize
}

fn get_exact_hwnd_insets(hwnd: HWND) -> WindowInset {
    unsafe {
        let mut window_rect = RECT::default();

        let _ = GetWindowRect(hwnd, &mut window_rect);
        let mut client_rect = RECT::default();
        let _ = GetClientRect(hwnd, &mut client_rect);

        let mut client_top_left = POINT {
            x: 0,
            y: 0,
        };
        let _ = ClientToScreen(hwnd, &mut client_top_left);

        let window_width = window_rect.right - window_rect.left;
        let client_width = client_rect.right - client_rect.left;
        let window_height = window_rect.bottom - window_rect.top;
        let client_height = client_rect.bottom - client_rect.top;

        let left_inset = window_width - client_width;
        let top_inset = window_height - client_height;

        WindowInset {
            x: left_inset / 2,
            y: top_inset / 2,
        }
    }
}

pub(crate) fn get_window_type(hwnd: HWND) -> WindowType {
    unsafe {
        let style = GetWindowLongPtrW(hwnd, GWL_STYLE) as u32;
        if (style & WS_CHILD.0) != 0 {
            return WindowType::Child;
        }

        if GetWindow(hwnd, GW_OWNER).is_ok() {
            return WindowType::Owned;
        }

        WindowType::Top
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

fn new_tab(app: &tauri::AppHandle, host_name: Option<&str>, window_handle: isize, label: &str) -> Tab {
    let host = host_name.unwrap_or(HOST.get().unwrap()).to_string();

    let window_labels = app.state::<Mutex<WindowLabels>>();
    let window_labels = window_labels.lock().unwrap();

    let (title, path) = if let Some(title) = window_labels.labels.get(label) {
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
        host,
        window_handle,
        label: label.to_string(),
        title,
        path,
        inset,
        style: unsafe { GetWindowLongPtrW(hwnd, GWL_STYLE) },
        parent,
        owner,
        bounds: Bounds::default(),
    }
}
