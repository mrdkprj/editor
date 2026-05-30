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
        use gtk::traits::{OverlayExt, WidgetExt};

        let host = app.get_webview_window("Main").unwrap();
        let vbox = host.default_vbox().unwrap();
        let host_chil = vbox.children();
        let overlay: gtk::Overlay = if host_chil.len() == 1 {
            let host_webview = host_chil.first().unwrap();
            host_webview.set_size_request(-1, 30);
            host_webview.set_vexpand(false);
            vbox.set_child_packing(host_webview, false, false, 0, gtk::PackType::Start);
            let overlay = gtk::Overlay::new();
            overlay.set_hexpand(true);
            overlay.set_vexpand(true);
            vbox.pack_start(&overlay, true, true, 0);
            overlay
        } else {
            use gtk::glib::Cast;

            let ov = host_chil.get(1).unwrap();
            ov.clone().downcast().unwrap()
        };

        for child_label in labels {
            let child = app.get_webview_window(&child_label).unwrap();
            let cbvos = child.default_vbox().unwrap();
            let chil = cbvos.children();
            let tab_webview = chil.first().unwrap();
            tab_webview.set_widget_name(&child_label);
            cbvos.remove(tab_webview);
            overlay.add_overlay(tab_webview);

            tab_webview.set_hexpand(true);
            tab_webview.set_vexpand(true);
            let _ = child.hide();
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
            use gtk::traits::ContainerExt;

            let host = app.get_webview_window("Main").unwrap();
            let vbox = host.default_vbox().unwrap();
            let host_chil = vbox.children();
            if host_chil.len() > 1 {
                use gtk::glib::Cast;

                let ov = host_chil.get(1).unwrap();
                let overlay: gtk::Overlay = ov.clone().downcast().unwrap();
                for widget in overlay.children() {
                    use gtk::traits::WidgetExt;

                    if widget.widget_name() == label {
                        use gtk::traits::BoxExt;

                        overlay.remove(&widget);

                        let cbvos = window.default_vbox().unwrap();
                        cbvos.pack_start(&widget, true, true, 0);
                        window.show().unwrap();
                    }
                }
            }
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
