use crate::watcher::{WatchTx, WatcherCommand};
use dialog::DialogOptions;
use serde::{Deserialize, Serialize};
use std::{env, path::PathBuf};
#[cfg(target_os = "windows")]
use tauri::Emitter;
use tauri::{AppHandle, Manager, WebviewWindow};
use zouni::*;
mod dialog;
mod fgrep;
mod helper;
mod menu;
mod tab;
mod watcher;

#[cfg(target_os = "linux")]
fn get_window_handle(window: &WebviewWindow) -> isize {
    use gtk::{ffi::GtkApplicationWindow, glib::translate::ToGlibPtr};

    let ptr: *mut GtkApplicationWindow = window.gtk_window().unwrap().to_glib_none().0;
    ptr as isize
}

#[cfg(target_os = "windows")]
fn get_window_handle(window: &WebviewWindow) -> isize {
    window.hwnd().unwrap().0 as _
}

#[tauri::command]
fn exists(payload: String) -> bool {
    PathBuf::from(payload).exists()
}

#[tauri::command]
fn is_file(payload: String) -> bool {
    PathBuf::from(payload).is_file()
}

#[tauri::command]
fn is_uris_available() -> bool {
    clipboard::is_uris_available()
}

#[tauri::command]
fn read_uris(window: WebviewWindow) -> Result<ClipboardData, String> {
    clipboard::read_uris(get_window_handle(&window))
}

#[tauri::command]
fn read_clipboard_text(window: WebviewWindow) -> Result<String, String> {
    clipboard::read_text(get_window_handle(&window))
}

#[tauri::command]
fn write_clipboard_text(window: WebviewWindow, payload: String) -> Result<(), String> {
    clipboard::write_text(get_window_handle(&window), payload)
}

#[tauri::command]
fn mkdir(payload: String) -> Result<(), String> {
    std::fs::create_dir(payload).map_err(|e| e.to_string())
}

#[tauri::command]
fn mkdir_all(payload: String) -> Result<(), String> {
    std::fs::create_dir_all(payload).map_err(|e| e.to_string())
}

#[tauri::command]
fn create(payload: String) -> Result<(), String> {
    match std::fs::File::create_new(payload) {
        Ok(_) => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
fn read_text_file(payload: String) -> Result<helper::ReadResult, String> {
    helper::read_to_string(&payload).map_err(|e| e.to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(non_snake_case)]
struct WriteFileInfo {
    fullPath: String,
    data: String,
    encoding: Option<String>,
}

#[tauri::command]
fn write_text_file(payload: WriteFileInfo) -> Result<(), String> {
    helper::write_to_file(payload)
}

#[tauri::command]
fn prepare_menu(window: WebviewWindow) {
    let label = window.label().to_string();
    let window_handle = get_window_handle(&window);
    menu::create(window.app_handle(), label, window_handle);
}

#[tauri::command]
fn change_theme(window: WebviewWindow, payload: String) {
    let (tauri_theme, menu_theme) = match payload.as_str() {
        "dark" => (tauri::Theme::Dark, wcpopup::config::Theme::Dark),
        "light" => (tauri::Theme::Light, wcpopup::config::Theme::Light),
        _ => (tauri::Theme::Light, wcpopup::config::Theme::System),
    };
    if let Some(main) = window.get_webview_window("Main") {
        let _ = main.set_theme(Some(tauri_theme));
    }
    let _ = window.set_theme(Some(tauri_theme));
    menu::change_menu_theme(window.app_handle(), menu_theme);
}

#[tauri::command]
async fn open_list_context_menu(app: AppHandle, payload: menu::OpenContextMenuRequest) {
    #[cfg(target_os = "windows")]
    {
        menu::popup_menu(&app, payload).await;
    }
    #[cfg(target_os = "linux")]
    {
        let app_handle = app.clone();
        app_handle
            .run_on_main_thread(move || {
                gtk::glib::spawn_future_local(async move {
                    menu::popup_menu(&app, payload).await;
                });
            })
            .unwrap();
    }
}

#[tauri::command]
async fn watch(app: AppHandle, payload: String) -> Result<(), String> {
    if let Some(tx) = app.try_state::<WatchTx>() {
        tx.inner().0.send(WatcherCommand::Watch(payload)).await.map_err(|e| e.to_string())
    } else {
        Ok(())
    }
}

#[tauri::command]
async fn unwatch(app: AppHandle, payload: String) -> Result<(), String> {
    if let Some(tx) = app.try_state::<WatchTx>() {
        tx.inner().0.send(WatcherCommand::Unwatch(payload)).await.map_err(|e| e.to_string())
    } else {
        Ok(())
    }
}

#[tauri::command]
async fn message(payload: DialogOptions) -> zouni::dialog::MessageResult {
    dialog::show(payload).await
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OpenFileResult {
    file_path: String,
    content: String,
    encoding: String,
}
#[tauri::command]
async fn show_open_dialog(payload: DialogOptions) -> Option<OpenFileResult> {
    let result = dialog::show_file_dialog(payload).await;
    if let Some(file_path) = result {
        let read_result = helper::read_to_string(&file_path).unwrap();
        Some(OpenFileResult {
            file_path,
            content: read_result.content,
            encoding: read_result.encoding,
        })
    } else {
        None
    }
}

#[tauri::command]
async fn show_save_dialog(payload: DialogOptions) -> Option<String> {
    dialog::show_save_dialog(payload).await
}

/*
   Must be async for tauri::WebviewWindowBuilder::from_config.
   On Windows, this function deadlocks when used in a synchronous command or event handlers, see the Webview2 issue. You should use async commands and separate threads when creating windows.
*/
#[cfg(target_os = "windows")]
#[tauri::command]
async fn new_window(app: AppHandle, payload: Vec<String>) {
    let mut args = vec!["thisapp".to_string()];
    if !payload.is_empty() {
        args.extend(payload);
    }
    helper::handle_second_instance(&app, args);
}

#[cfg(target_os = "linux")]
#[tauri::command]
fn new_window(app: AppHandle, payload: Vec<String>) {
    let mut args = vec!["thisapp".to_string()];
    if !payload.is_empty() {
        args.extend(payload);
    }
    helper::handle_second_instance(&app, args);
}

#[tauri::command]
fn get_args(app: AppHandle) -> Result<helper::InitArgs, String> {
    helper::get_init_args(app)
}

#[allow(unused_variables)]
#[tauri::command]
fn register_drop_target(window: WebviewWindow) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        zouni::drag_drop::register(window.hwnd().unwrap().0 as isize)
    }
    #[cfg(target_os = "linux")]
    {
        Ok(())
    }
}

#[allow(unused_variables)]
#[tauri::command]
fn listen_file_drop(window: WebviewWindow, app: AppHandle, payload: Option<String>) -> tauri::Result<()> {
    #[cfg(target_os = "windows")]
    {
        let label = window.label().to_string();
        window.with_webview(move |webview| {
            zouni::webview2::register_file_drop(unsafe { &webview.controller().CoreWebView2().unwrap() }, payload, move |event| {
                app.emit_to(
                    tauri::EventTarget::WebviewWindow {
                        label: label.clone(),
                    },
                    "tauri://drag-drop",
                    event,
                )
                .unwrap();
            })
            .unwrap();
        })
    }
    #[cfg(target_os = "linux")]
    {
        Ok(())
    }
}

#[tauri::command]
fn unlisten_file_drop() {
    #[cfg(target_os = "windows")]
    zouni::webview2::clear();
}

#[tauri::command]
async fn run_grep(window: WebviewWindow, payload: fgrep::GrepRequest) -> Result<Vec<fgrep::GrepResult>, String> {
    fgrep::run_grep(window.app_handle(), window.label(), payload).await
}

#[tauri::command]
async fn abort_grep() {
    fgrep::cancel().await;
}

#[tauri::command]
fn change_encoding(payload: helper::EncodeArg) -> Result<String, String> {
    helper::encode(payload)
}

#[tauri::command]
fn to_child_window(app: tauri::AppHandle, payload: Vec<String>) {
    tab::to_child_window(app, payload);
}

#[tauri::command]
fn restore_webview(app: tauri::AppHandle, payload: String) {
    tab::restore_webview(app, payload);
}

#[tauri::command]
fn get_webview_labels(app: tauri::AppHandle) -> helper::OpenedWebview {
    helper::get_webview_labels(app)
}

#[tauri::command]
fn update_webview_label(window: WebviewWindow, payload: helper::WebviewTitle) {
    helper::update_webview_label(window.app_handle(), payload);
}

#[tauri::command]
fn is_file_opened(app: tauri::AppHandle, payload: String) -> Option<String> {
    helper::is_file_opened(&app, Some(payload))
}

#[cfg(target_os = "linux")]
#[tauri::command]
fn bring_to_front(app: tauri::AppHandle, payload: String) {
    tab::bring_to_front(app, payload)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app_handel, args, _| helper::handle_second_instance(app_handel, args)))
        .setup(|app| {
            let args: Vec<String> = env::args().collect();
            helper::start(app.app_handle());
            helper::setup(app.app_handle(), args);

            #[cfg(target_os = "linux")]
            {
                let window = app.get_webview_window("Main").unwrap();
                let label = window.label().to_string();
                let window_handle = get_window_handle(&window);
                menu::create(app.app_handle(), label, window_handle);
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::Destroyed = event {
                helper::remove_from_webview_label(window.app_handle(), window.label())
            }
        })
        .invoke_handler(tauri::generate_handler![
            prepare_menu,
            open_list_context_menu,
            is_file_opened,
            exists,
            is_file,
            is_uris_available,
            read_uris,
            read_clipboard_text,
            write_clipboard_text,
            mkdir,
            mkdir_all,
            create,
            read_text_file,
            write_text_file,
            watch,
            unwatch,
            message,
            show_open_dialog,
            show_save_dialog,
            get_args,
            register_drop_target,
            new_window,
            listen_file_drop,
            unlisten_file_drop,
            change_theme,
            run_grep,
            abort_grep,
            change_encoding,
            restore_webview,
            to_child_window,
            #[cfg(target_os = "linux")]
            bring_to_front,
            get_webview_labels,
            update_webview_label,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
