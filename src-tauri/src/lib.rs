use crate::watcher::{WatchTx, WatcherCommand};
use dialog::DialogOptions;
use serde::{Deserialize, Serialize};
use std::{env, path::PathBuf};
use tauri::{AppHandle, Emitter, Manager, WebviewWindow};
use zouni::*;
mod dialog;
mod fgrep;
mod helper;
mod menu;
mod session;
mod watcher;

#[cfg(target_os = "linux")]
fn get_window_handel(window: &WebviewWindow) -> isize {
    use gtk::{ffi::GtkApplicationWindow, glib::translate::ToGlibPtr};

    let ptr: *mut GtkApplicationWindow = window.gtk_window().unwrap().to_glib_none().0;
    ptr as isize
}

#[cfg(target_os = "windows")]
fn get_window_handel(window: &WebviewWindow) -> isize {
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
    clipboard::read_uris(get_window_handel(&window))
}

#[tauri::command]
fn read_clipboard_text(window: WebviewWindow) -> Result<String, String> {
    clipboard::read_text(get_window_handel(&window))
}

#[tauri::command]
fn write_clipboard_text(window: WebviewWindow, payload: String) -> Result<(), String> {
    clipboard::write_text(get_window_handel(&window), payload)
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
    let window_handle = get_window_handel(&window);
    menu::create(window.app_handle(), window_handle);
}

#[tauri::command]
fn change_theme(window: WebviewWindow, payload: String) {
    let (tauri_them, menu_theme) = match payload.as_str() {
        "dark" => (tauri::Theme::Dark, wcpopup::config::Theme::Dark),
        "light" => (tauri::Theme::Light, wcpopup::config::Theme::Light),
        _ => (tauri::Theme::Light, wcpopup::config::Theme::System),
    };
    let _ = window.set_theme(Some(tauri_them));
    menu::change_menu_theme(window.app_handle(), menu_theme);
}

#[tauri::command]
async fn open_list_context_menu(window: WebviewWindow, payload: menu::Position) {
    #[cfg(target_os = "windows")]
    {
        menu::popup_menu(window.app_handle(), payload).await;
    }
    #[cfg(target_os = "linux")]
    {
        window
            .run_on_main_thread(move || {
                gtk::glib::spawn_future_local(async move {
                    menu::popup_menu(window.app_handle(), payload).await;
                });
            })
            .unwrap();
    }
}

#[tauri::command]
fn watch(app: AppHandle, payload: String) -> Result<(), String> {
    if let Some(tx) = app.try_state::<WatchTx>() {
        tx.inner().0.send(WatcherCommand::Watch(payload)).map_err(|e| e.to_string())
    } else {
        Ok(())
    }
}

#[tauri::command]
fn unwatch(app: AppHandle, payload: String) -> Result<(), String> {
    if let Some(tx) = app.try_state::<WatchTx>() {
        tx.inner().0.send(WatcherCommand::Unwatch(payload)).map_err(|e| e.to_string())
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

#[tauri::command]
fn new_window(app: AppHandle, payload: Option<String>) -> Result<(), String> {
    let app_path = tauri::process::current_binary(&app.env()).map_err(|e| e.to_string())?;
    if cfg!(windows) {
        zouni::shell::open_path_with(payload.unwrap_or_default(), app_path)
    } else {
        let payload = payload.unwrap_or_default();
        let args: Vec<&str> = payload.split(" ").collect();
        std::process::Command::new(app_path).args(args).spawn().map_err(|e| e.to_string())?;
        Ok(())
    }
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
                app.get_webview_window(&label).unwrap().emit("tauri://drag-drop", event).unwrap();
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
    fgrep::run_grep(&window, payload).await
}

#[tauri::command]
fn abort_grep() {
    fgrep::cancel();
}

#[tauri::command]
fn change_encoding(payload: helper::EncodeArg) -> Result<String, String> {
    helper::encode(payload)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let args: Vec<String> = env::args().collect();
            helper::setup(app, args);
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::Destroyed = event {
                helper::exit(window.app_handle());
            }
        })
        .invoke_handler(tauri::generate_handler![
            prepare_menu,
            open_list_context_menu,
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
