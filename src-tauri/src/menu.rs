use serde::{Deserialize, Serialize};
use tauri::{Emitter, Manager};
use wcpopup::{
    config::{ColorScheme, Config, MenuSize, Theme, ThemeColor, DEFAULT_DARK_COLOR_SCHEME},
    Menu, MenuBuilder,
};

const MENU_EVENT_NAME: &str = "contextmenu_event";

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

struct AppMenu(Menu);

pub async fn popup_menu(app_handle: &tauri::AppHandle, position: Position) {
    let menu = &app_handle.state::<AppMenu>().inner().0;

    let result = menu.popup_at_async(position.x, position.y).await;

    if let Some(item) = result {
        app_handle
            .emit(
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

pub fn create(app_handle: &tauri::AppHandle, window_handle: isize) {
    let menu = create_list_menu(window_handle);
    app_handle.manage(AppMenu(menu));
}

pub fn change_menu_theme(app_handle: &tauri::AppHandle, theme: Theme) {
    let menu = &app_handle.state::<AppMenu>().inner().0;
    menu.set_theme(theme);
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
