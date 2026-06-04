use serde::{Deserialize, Serialize};
use smol::lock::Mutex;
use std::collections::HashMap;
use tauri::{Emitter, Manager};
use wcpopup::{
    config::{ColorScheme, Config, MenuSize, Theme, ThemeColor, DEFAULT_DARK_COLOR_SCHEME},
    Menu, MenuBuilder,
};

const MENU_EVENT_NAME: &str = "contextmenu_event";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppMenu(HashMap<String, Menu>);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenContextMenuRequest {
    opener: String,
    receiver: String,
    position: Position,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    x: i32,
    y: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ContextMenuEvent {
    id: String,
    value: Option<String>,
}

pub async fn popup_menu(app_handle: &tauri::AppHandle, e: OpenContextMenuRequest) {
    let state = app_handle.state::<Mutex<AppMenu>>();
    let state = state.lock().await;
    let menu = state.0.get(&e.opener).unwrap();
    let result = menu.popup_at_async(e.position.x, e.position.y).await;

    if let Some(item) = result {
        app_handle
            .emit_to(
                tauri::EventTarget::WebviewWindow {
                    label: e.receiver,
                },
                MENU_EVENT_NAME,
                ContextMenuEvent {
                    id: item.id,
                    value: None,
                },
            )
            .unwrap();
    };
}

fn get_menu_config(theme: Theme) -> Config {
    Config {
        theme,
        color: ThemeColor {
            dark: ColorScheme {
                color: 0xefefef,
                background_color: 0x202020,
                ..DEFAULT_DARK_COLOR_SCHEME
            },
            ..Default::default()
        },
        size: MenuSize {
            item_horizontal_padding: 20,
            ..Default::default()
        },
        ..Default::default()
    }
}

pub fn create(app_handle: &tauri::AppHandle, label: String, window_handle: isize) {
    let menu = create_list_menu(window_handle);
    let state = app_handle.state::<Mutex<AppMenu>>();
    let mut state = state.try_lock().unwrap();
    let _ = state.0.insert(label, menu);
}

pub fn remove(app_handle: &tauri::AppHandle, label: &str) {
    let state = app_handle.state::<Mutex<AppMenu>>();
    let mut state = state.try_lock().unwrap();
    let _ = state.0.remove(label);
}

pub fn change_menu_theme(app_handle: &tauri::AppHandle, theme: Theme) {
    let state = app_handle.state::<Mutex<AppMenu>>();
    let state = state.try_lock().unwrap();
    for key in state.0.keys() {
        let menu = state.0.get(key).unwrap();
        menu.set_theme(theme);
    }
}

fn create_list_menu(window_handle: isize) -> Menu {
    let config = get_menu_config(Theme::System);
    let mut builder = MenuBuilder::new_from_config(window_handle, config);
    builder.text_with_accelerator("Copy", "Copy", false, "Ctrl+C");
    builder.text_with_accelerator("Cut", "Cut", false, "Ctrl+X");
    builder.text_with_accelerator("Paste", "Paste", false, "Ctrl+V");
    builder.text("copyFilePath", "Copy File Path", false);
    builder.separator();
    let mut sub = builder.submenu("Convert", "Convert", false);
    sub.text("transformToLowercase", "To Lowercase", false);
    sub.text("transformToUppercase", "To Uppercase", false);
    sub.text("transformToSnakecase", "To Snakecase", false);
    sub.text("transformToCamelcase", "To Camelcase", false);
    sub.text("transformToPascalcase", "To Pascalcase", false);
    sub.text("transformToTitlecase", "To Titlecase", false);
    sub.text("transformToKebabcase", "To Kebabcase", false);
    sub.build().unwrap();
    builder.separator();
    builder.text("Format", "Format", false);

    builder.build().unwrap()
}
