use tauri::Manager;
#[cfg(target_os = "windows")]
use windows::Win32::{
    Foundation::HWND,
    UI::WindowsAndMessaging::{GetWindowLongPtrW, SetParent, SetWindowLongPtrW, GWL_STYLE, WS_CHILD, WS_POPUP},
};

pub fn to_child_window(app: tauri::AppHandle, labels: Vec<String>) {
    #[cfg(target_os = "windows")]
    {
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
    #[cfg(target_os = "linux")]
    {
        use gtk::traits::{BoxExt, ContainerExt};

        let host = app.get_webview_window("Main").unwrap();
        let size = host.inner_size().unwrap();

        for child_label in labels {
            use gtk::traits::{OverlayExt, WidgetExt};

            let host = app.get_webview_window("Main").unwrap();
            let vbox = host.default_vbox().unwrap();
            // let host_chil = vbox.children();
            // let init_web = host_chil.first().unwrap();
            // init_web.set_size_request(size.width as _, size.height as _);
            // init_web.set_vexpand(true);
            // init_web.set_hexpand(true);

            let child = app.get_webview_window(&child_label).unwrap();

            let cbvos = child.default_vbox().unwrap();
            let chil = cbvos.children();
            let web = chil.first().unwrap();
            cbvos.remove(web);
            let overlay = gtk::Overlay::new();
            overlay.add_overlay(web);
            vbox.pack_start(&overlay, true, true, 0);
            vbox.reorder_child(&overlay, 1);
            // web.hexpands();
            // web.vexpands();
            // web.set_vexpand(true);
            // web.set_hexpand(true);
            // web.set_margin_start(5);
            // web.set_margin_end(5);
            // web.set_margin_top(30);
            // web.set_size_request(size.width as i32, size.height as i32 - 30);
            child.set_decorations(true).unwrap();
            let _ = child.hide();
            // use gtk::traits::{GtkWindowExt, WidgetExt};

            // let host = app.get_webview_window("Main").unwrap();
            // let host_window = host.gtk_window().unwrap();
            // let child = app.get_webview_window(&child_label).unwrap();
            // let child_window = child.gtk_window().unwrap();

            // child_window.set_opacity(0.0);

            // child_window.set_transient_for(Some(&host_window));
        }
    }
}

pub fn restore_webview(app: tauri::AppHandle, label: String) {
    if let Some(window) = app.get_webview_window(&label) {
        #[cfg(target_os = "windows")]
        {
            let hwnd = window.hwnd().unwrap();

            let style = unsafe { GetWindowLongPtrW(hwnd, GWL_STYLE) };
            unsafe { SetWindowLongPtrW(hwnd, GWL_STYLE, (style & !WS_CHILD.0 as isize) | WS_POPUP.0 as isize) };

            unsafe { SetParent(hwnd, None).unwrap() };
        }
        #[cfg(target_os = "linux")]
        {
            use gtk::traits::{GtkWindowExt, WidgetExt};

            let child_window = window.gtk_window().unwrap();
            child_window.set_opacity(1.0);
            child_window.set_transient_for(gtk::Window::NONE);
        }
    }
}

#[cfg(target_os = "linux")]
pub fn make_opaque(app: tauri::AppHandle, label: String) {
    if let Some(window) = app.get_webview_window(&label) {
        {
            use gtk::traits::{GtkWindowExt, WidgetExt};

            let child_window = window.gtk_window().unwrap();
            println!("{:?}", child_window.position());
            child_window.set_opacity(1.0);
        }
    }
}
