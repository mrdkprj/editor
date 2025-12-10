use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use tauri::{Emitter, EventTarget, WebviewWindow};
use wcpopup::{
    config::{ColorScheme, Config, MenuSize, Theme, ThemeColor, DEFAULT_DARK_COLOR_SCHEME},
    Menu, MenuBuilder,
};

static MENU: OnceCell<Menu> = OnceCell::new();
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

pub async fn popup_menu(window: &WebviewWindow, position: Position) {
    let menu = MENU.get().unwrap();

    let result = menu.popup_at_async(position.x, position.y).await;

    if let Some(item) = result {
        window
            .emit_to(
                EventTarget::WebviewWindow {
                    label: window.label().to_string(),
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

pub fn create(window_handle: isize) {
    create_list_menu(window_handle);
}

pub fn change_menu_theme(theme: Theme) {
    MENU.get().unwrap().set_theme(theme);
}

fn create_list_menu(window_handle: isize) {
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

    let menu = builder.build().unwrap();

    MENU.get_or_init(|| menu);
}
