use crate::{
    helper::WindowLabels,
    tab::{emit, emit_to, Bounds, ModeChangedArg, Tab, TabEvent, TabState, Title, WebviewTitle, WindowInset, WindowMode, TAB_WINDOW_LABEL},
};
use std::{collections::HashMap, sync::Mutex, time::Duration};
use tauri::{Manager, WebviewWindow};
use windows::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM},
    Graphics::Gdi::ClientToScreen,
    UI::{
        Input::KeyboardAndMouse::SetFocus,
        Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass},
        WindowsAndMessaging::*,
    },
};

const OFF_SCREEN: i32 = -30000;

#[derive(Debug, PartialEq)]
pub(crate) enum WindowType {
    Top,
    Child,
    Owned,
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
    let host = app.get_webview_window(TAB_WINDOW_LABEL).unwrap();
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

    if let Some((index, tab)) = state.tabs.iter().enumerate().find(|(_, tab)| tab.label == label) {
        after_detach(tab.window_handle);

        if state.tabs.len() == 1 {
            /* If this is the last tab, hide the host */
            let host = app.get_webview_window(TAB_WINDOW_LABEL).unwrap();
            remove_subclass(host.hwnd().unwrap());
            let _ = unsafe { SetWindowPos(host.hwnd().unwrap(), None, OFF_SCREEN, OFF_SCREEN, 0, 0, SWP_NOSIZE) };
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
    let host = app.get_webview_window(TAB_WINDOW_LABEL).unwrap();
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
    let _ = app.get_webview_window(TAB_WINDOW_LABEL).unwrap().minimize();
}

fn enter_tab_mode(app: &tauri::AppHandle, tabs: &mut [Tab], mode: &mut WindowMode, activator: &str) {
    let host = app.get_webview_window(TAB_WINDOW_LABEL).unwrap();

    unsafe {
        let current_style = GetWindowLongPtrW(host.hwnd().unwrap(), GWL_STYLE) as u32;
        if (current_style & WS_CLIPCHILDREN.0) == 0 {
            SetWindowLongPtrW(host.hwnd().unwrap(), GWL_STYLE, (current_style | WS_CLIPCHILDREN.0) as isize);
        }
    }

    /* Must show parent first. Otherwise, extra top margin shows */
    let _ = unsafe { SetWindowPos(host.hwnd().unwrap(), None, OFF_SCREEN, OFF_SCREEN, 0, 0, SWP_NOSIZE | SWP_NOZORDER | SWP_FRAMECHANGED) };
    host.unmaximize().unwrap();
    // host.with_webview(|w| unsafe { w.controller().SetIsVisible(false).unwrap() }).unwrap();
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

    bring_to_front(app, tabs, mode, activator);

    host.set_position(pos).unwrap();

    let _ = unsafe { SetWindowSubclass(host.hwnd().unwrap(), Some(proc), 200, Box::into_raw(Box::new(app.clone())) as usize) };
}

fn exit_tab_mode(app: &tauri::AppHandle, tabs: &[Tab], mode: &mut WindowMode) {
    mode.active_tab_label = String::new();

    let host = app.get_webview_window(TAB_WINDOW_LABEL).unwrap();

    for tab in tabs.iter() {
        detach_from_tab(tab, true);
        after_detach(tab.window_handle);
    }

    remove_subclass(host.hwnd().unwrap());
    host.hide().unwrap();
}

fn before_attach(window: &tauri::WebviewWindow) {
    /* On Windows, window must be shown before attach. Otherwise, focus can be moved to the parent */
    let _ = unsafe { SetWindowPos(window.hwnd().unwrap(), None, OFF_SCREEN, OFF_SCREEN, 0, 0, SWP_NOSIZE | SWP_SHOWWINDOW | SWP_FRAMECHANGED) };
}

fn after_detach(window_handle: isize) {
    let _ = unsafe { RemoveWindowSubclass(to_hwnd(window_handle), Some(child_proc), 300) };
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

    let _ = unsafe { SetWindowPos(child, Some(HWND_BOTTOM), -tab.inset.x, 0, width + tab.inset.x * 2, height + tab.inset.y * 2, SWP_FRAMECHANGED | SWP_NOACTIVATE) };
    let _ = unsafe { SetWindowSubclass(child, Some(child_proc), 300, Box::into_raw(Box::new(parent_window.app_handle().clone())) as usize) };
}

fn bring_to_front(_app: &tauri::AppHandle, tabs: &[Tab], mode: &mut WindowMode, label: &str) {
    if mode.active_tab_label == label {
        return;
    }

    if let Some(new) = tabs.iter().find(|tab| tab.label == label) {
        let _ = unsafe { SetWindowPos(to_hwnd(new.window_handle), Some(HWND_TOP), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE) };
        emit_to(_app, TabEvent::Activated, label);
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
    }
}

unsafe extern "system" fn proc(hwnd: HWND, umsg: u32, wparam: WPARAM, lparam: LPARAM, _uidsubclass: usize, dwrefdata: usize) -> LRESULT {
    if umsg == WM_EXITSIZEMOVE {
        let item_data_ptr = dwrefdata as *const tauri::AppHandle;
        let app = &*item_data_ptr;
        let mode = app.state::<Mutex<WindowMode>>();
        if let Ok(mode) = mode.try_lock() {
            on_resized(mode.window_handle);
        };
    }

    if umsg == WM_SIZE {
        if wparam.0 as u32 == SIZE_MINIMIZED {
            let item_data_ptr = dwrefdata as *const tauri::AppHandle;
            let app = &*item_data_ptr;
            let mode = app.state::<Mutex<WindowMode>>();
            if let Ok(mut mode) = mode.try_lock() {
                mode.minimized = true;
            };
        }

        if wparam.0 as u32 == SIZE_RESTORED {
            let item_data_ptr = dwrefdata as *const tauri::AppHandle;
            let app = &*item_data_ptr;
            let mode = app.state::<Mutex<WindowMode>>();
            if let Ok(mut mode) = mode.try_lock() {
                if mode.minimized {
                    mode.minimized = false;
                    on_restore(mode.window_handle);
                }
            };
        }
    }

    DefSubclassProc(hwnd, umsg, wparam, lparam)
}

fn get_resize_direction(flag: u32) -> &'static str {
    match flag {
        WMSZ_BOTTOM => "South",
        WMSZ_BOTTOMLEFT => "SouthWest",
        WMSZ_BOTTOMRIGHT => "SouthEast",
        WMSZ_LEFT => "West",
        WMSZ_RIGHT => "East",
        WMSZ_TOP => "North",
        WMSZ_TOPLEFT => "NorthWest",
        WMSZ_TOPRIGHT => "NorthEast",
        _ => "North",
    }
}

unsafe extern "system" fn child_proc(hwnd: HWND, umsg: u32, wparam: WPARAM, lparam: LPARAM, _uidsubclass: usize, dwrefdata: usize) -> LRESULT {
    if umsg == WM_WINDOWPOSCHANGING {
        let item_data_ptr = dwrefdata as *const tauri::AppHandle;
        let app = &*item_data_ptr;
        let mode = app.state::<Mutex<WindowMode>>();
        if let Ok(mode) = mode.try_lock() {
            if mode.resizing {
                let pos = &mut *(lparam.0 as *mut WINDOWPOS);
                pos.flags |= SWP_NOMOVE;
            }
        };
    }

    if umsg == WM_ENTERSIZEMOVE {
        let item_data_ptr = dwrefdata as *const tauri::AppHandle;
        let app = &*item_data_ptr;
        let mode = app.state::<Mutex<WindowMode>>();
        if let Ok(mut mode) = mode.try_lock() {
            mode.resizing = true;
            mode.propagated = false;
        };
    }

    if umsg == WM_EXITSIZEMOVE {
        let item_data_ptr = dwrefdata as *const tauri::AppHandle;
        let app = &*item_data_ptr;
        let mode = app.state::<Mutex<WindowMode>>();
        if let Ok(mut mode) = mode.try_lock() {
            mode.resizing = false;
            mode.propagated = false;
        };
    }

    if umsg == WM_SIZING {
        let item_data_ptr = dwrefdata as *const tauri::AppHandle;
        let app = &*item_data_ptr;
        let mode = app.state::<Mutex<WindowMode>>();
        if let Ok(mut mode) = mode.try_lock() {
            mode.resizing = false;
            if !mode.propagated {
                let direction = get_resize_direction(wparam.0 as u32);
                emit_to(app, TabEvent::DragResize(direction.to_string()), &mode.active_tab_label);
                mode.propagated = true;
            }
        };
    }

    DefSubclassProc(hwnd, umsg, wparam, lparam)
}

fn remove_subclass(hwnd: HWND) {
    let _ = unsafe { RemoveWindowSubclass(hwnd, Some(proc), 200) };
}

pub fn on_resizing(tabs: &[Tab], width: i32, height: i32) {
    for tab in tabs {
        let _ = unsafe {
            SetWindowPos(to_hwnd(tab.window_handle), None, 0, 0, width + tab.inset.x * 2, height + tab.inset.y * 2, SWP_NOMOVE | SWP_NOZORDER | SWP_NOCOPYBITS | SWP_NOACTIVATE | SWP_NOSENDCHANGING)
        };
    }
}

fn on_resized(window_handle: isize) {
    let _ = unsafe { SetWindowPos(to_hwnd(window_handle), Some(HWND_TOP), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE) };
}

fn on_restore(window_handle: isize) {
    /* Restore z order after 5 millisecs when the parent is restored from minimized state */
    smol::spawn(async move {
        smol::Timer::after(Duration::from_millis(5)).await;
        on_resized(window_handle);
    })
    .detach();
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
        active: false,
        bounds: Bounds::default(),
    }
}
