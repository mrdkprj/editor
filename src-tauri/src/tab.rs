#[cfg(target_os = "linux")]
use gtk::{
    glib::Cast,
    traits::{BoxExt, ContainerExt, OverlayExt, WidgetExt},
};
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
}

#[cfg(target_os = "linux")]
pub fn bring_to_front(app: tauri::AppHandle, label: String) {
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
