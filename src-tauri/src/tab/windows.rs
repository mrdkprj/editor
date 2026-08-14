use crate::{
    helper::WindowLabels,
    tab::{emit, emit_to, Bounds, ModeChangedArg, Tab, TabEvent, TabState, Title, WebviewTitle, WindowInset, WindowMode, HOST},
};
use std::{collections::HashMap, sync::Mutex, time::Duration};
use tauri::{Manager, WebviewWindow};
use windows::{
    core::{Free, PCWSTR},
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, POINT, POINTS, RECT, WPARAM},
        Graphics::Gdi::{ClientToScreen, CreateRectRgn, GetWindowRgn, SetWindowRgn},
        UI::{
            Input::KeyboardAndMouse::{ReleaseCapture, SetFocus},
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

#[derive(Debug, PartialEq)]
pub(crate) enum WindowType {
    Top,
    Child,
    Owned,
}

pub fn prepare(app: &tauri::AppHandle) {
    let host = app.get_webview_window(HOST.get().unwrap()).unwrap();
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
        if unsafe { GetWindowRgn(child, region) } != windows::Win32::Graphics::Gdi::RGN_ERROR {
            children.push(child.0 as isize);
        }
        unsafe { region.free() };
        start_child = child;
    }

    if children.is_empty() || children.len() > 1 {
        panic!("Can't find undecorated_resize window");
    }

    mode.undecorated_resize = *children.first().unwrap();
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
    let host = app.get_webview_window(HOST.get().unwrap()).unwrap();
    let size = host.inner_size().unwrap();

    before_attach(window);
    attach_to_tab(&host, tab, size.width as _, size.height as _);
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
        let position = app.get_webview_window(HOST.get().unwrap()).unwrap().outer_position().unwrap();
        /* Set parent's position */
        let _ = unsafe { SetWindowPos(to_hwnd(tab.window_handle), Some(HWND_BOTTOM), position.x, position.y, tab.bounds.width as _, tab.bounds.height as _, SWP_HIDEWINDOW) };
        detach_from_tab(tab, false);

        let mode = app.state::<Mutex<WindowMode>>();
        let mut mode = mode.lock().unwrap();

        if state.tabs.len() == 1 {
            /* If this is the last tab, hide the host */
            let host = app.get_webview_window(HOST.get().unwrap()).unwrap();
            let _ = unsafe { SetWindowPos(host.hwnd().unwrap(), None, OFF_SCREEN, OFF_SCREEN, 0, 0, SWP_NOSIZE) };
            uninstall_subclass(host.hwnd().unwrap(), mode.undecorated_resize);
            let _ = host.hide();
        } else {
            /* Change active tab only instead of changing child to top-level window */
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

fn install_subclass(app: &tauri::AppHandle, host: HWND, undecorated_resize: isize) {
    unsafe {
        let current_style = GetWindowLongPtrW(host, GWL_STYLE) as u32;
        if (current_style & WS_CLIPCHILDREN.0) == 0 {
            SetWindowLongPtrW(host, GWL_STYLE, (current_style | WS_CLIPCHILDREN.0) as isize);
        }
    }

    let _ = unsafe { SetWindowSubclass(host, Some(subclass_parent), PARENT_SUBCLASS_ID, Box::into_raw(Box::new(app.clone())) as _) };
    let _ = unsafe { SetWindowSubclass(to_hwnd(undecorated_resize), Some(resize_subclass), RESIZE_SUBCLASS_ID, host.0 as _) };
}

fn uninstall_subclass(host: HWND, undecorated_resize: isize) {
    let _ = unsafe { RemoveWindowSubclass(to_hwnd(undecorated_resize), Some(resize_subclass), RESIZE_SUBCLASS_ID) };
    let _ = unsafe { RemoveWindowSubclass(host, Some(subclass_parent), PARENT_SUBCLASS_ID) };
}

fn enter_tab_mode(app: &tauri::AppHandle, tabs: &mut [Tab], mode: &mut WindowMode, activator: &str) {
    let host = app.get_webview_window(HOST.get().unwrap()).unwrap();
    install_subclass(app, host.hwnd().unwrap(), mode.undecorated_resize);

    let activator_window = app.get_webview_window(activator).unwrap();
    let size = activator_window.outer_size().unwrap();
    let pos = activator_window.outer_position().unwrap();

    for tab in tabs.iter_mut() {
        let child = app.get_webview_window(&tab.label).unwrap();
        let bounds = get_bounds(&child);
        tab.bounds = bounds;
        before_attach(&child);
        attach_to_tab(&host, tab, size.width as _, size.height as _);
    }

    bring_to_front(app, tabs, mode, activator);

    host.set_size(size).unwrap();
    host.set_position(pos).unwrap();
    host.unmaximize().unwrap();
    host.show().unwrap();
}

fn exit_tab_mode(app: &tauri::AppHandle, tabs: &[Tab], mode: &mut WindowMode) {
    let host = app.get_webview_window(HOST.get().unwrap()).unwrap();
    uninstall_subclass(host.hwnd().unwrap(), mode.undecorated_resize);

    mode.active_tab_label = String::new();

    let _ = host.hide();

    for tab in tabs.iter() {
        detach_from_tab(tab, true);
    }
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

fn bring_to_front(app: &tauri::AppHandle, tabs: &[Tab], mode: &mut WindowMode, label: &str) {
    if mode.active_tab_label == label {
        return;
    }

    if let Some(new) = tabs.iter().find(|tab| tab.label == label) {
        let _ = unsafe { SetWindowPos(to_hwnd(new.window_handle), Some(HWND_TOP), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE) };
        emit_to(app, TabEvent::Activated, label);
        let _ = unsafe { SetFocus(Some(to_hwnd(new.window_handle))) };

        mode.active_tab_label = label.to_string();
        mode.window_handle = new.window_handle;
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
                bring_to_front(&app, &state.tabs, &mut mode, &tab.label);
                emit(
                    &app,
                    TabEvent::Added(Title {
                        label: tab.label,
                        title: tab.title,
                        path: tab.path,
                    }),
                    None,
                );
            };
        };
    })
    .detach();
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
            let item_data_ptr = dwrefdata as *const tauri::AppHandle;
            let app = &*item_data_ptr;
            let state = app.state::<Mutex<TabState>>();
            if let Ok(state) = state.try_lock() {
                on_resized(&state.tabs, width, height);
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

unsafe extern "system" fn child_proc(hwnd: HWND, umsg: u32, wparam: WPARAM, lparam: LPARAM, _uidsubclass: usize, dwrefdata: usize) -> LRESULT {
    if umsg == WM_NCLBUTTONDOWN {
        let hit_test = wparam.0 as u32;

        let is_resize_edge = matches!(hit_test, HTTOP | HTBOTTOM | HTLEFT | HTRIGHT | HTTOPLEFT | HTTOPRIGHT | HTBOTTOMLEFT | HTBOTTOMRIGHT);

        if is_resize_edge {
            let item_data_ptr = dwrefdata as *const tauri::AppHandle;
            let app = &*item_data_ptr;
            if let Ok(parent_hwnd) = app.get_webview_window(HOST.get().unwrap()).unwrap().hwnd() {
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

pub fn on_resized(tabs: &[Tab], width: i32, height: i32) {
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
        bounds: Bounds::default(),
    }
}
