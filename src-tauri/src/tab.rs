use tauri::Manager;
use windows::Win32::{
    Foundation::HWND,
    UI::WindowsAndMessaging::{GetWindowLongPtrW, SetParent, SetWindowLongPtrW, GWL_STYLE, WS_CHILD, WS_POPUP},
};

pub fn to_child_window(app: tauri::AppHandle, labels: Vec<String>) {
    let host = app.get_webview_window("Main").unwrap();
    let hwnd = host.hwnd().unwrap();
    let host_hwnd_ptr = hwnd.0 as isize;

    for child_label in labels {
        let child = app.get_webview_window(&child_label).unwrap();
        let child_hwnd = child.hwnd().unwrap();

        let mut style = unsafe { GetWindowLongPtrW(child_hwnd, GWL_STYLE) } as u32;
        style &= !(WS_POPUP.0);
        style |= WS_CHILD.0;
        unsafe { SetWindowLongPtrW(child_hwnd, GWL_STYLE, style as isize) };

        unsafe { SetParent(child_hwnd, Some(HWND(host_hwnd_ptr as *mut std::ffi::c_void))).unwrap() };
    }
}

pub fn restore_webview(app: tauri::AppHandle, label: String) {
    if let Some(window) = app.get_webview_window(&label) {
        let hwnd = window.hwnd().unwrap();

        let style = unsafe { GetWindowLongPtrW(hwnd, GWL_STYLE) };
        unsafe { SetWindowLongPtrW(hwnd, GWL_STYLE, (style & !WS_CHILD.0 as isize) | WS_POPUP.0 as isize) };

        unsafe { SetParent(hwnd, None).unwrap() };
    }
}
