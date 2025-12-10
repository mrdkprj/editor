use serde::{Deserialize, Serialize};
use zouni::dialog::{message, open, save, MessageDialogKind, MessageDialogOptions, MessageResult, OpenDialogOptions, OpenProperty, SaveDialogOptions};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogOptions {
    dialog_type: String,
    title: Option<String>,
    kind: Option<String>,
    ok_label: Option<String>,
    cancel_label: Option<String>,
    message: String,
    default_path: Option<String>,
}

pub async fn show(info: DialogOptions) -> MessageResult {
    match info.dialog_type.as_str() {
        "message" => show_message(info).await,
        "confirm" => show_confirm(info).await,
        "ask" => show_ask(info).await,
        _ => MessageResult::default(),
    }
}

fn get_level(kind: &Option<String>) -> MessageDialogKind {
    if let Some(kind) = kind {
        match kind.as_str() {
            "info" => MessageDialogKind::Info,
            "warning" => MessageDialogKind::Warning,
            "error" => MessageDialogKind::Error,
            _ => MessageDialogKind::Info,
        }
    } else {
        MessageDialogKind::Info
    }
}

async fn show_message(info: DialogOptions) -> MessageResult {
    let options = MessageDialogOptions {
        title: info.title,
        kind: Some(get_level(&info.kind)),
        buttons: Vec::new(),
        message: info.message,
        cancel_id: None,
    };
    message(options).await
}

async fn show_confirm(info: DialogOptions) -> MessageResult {
    let options = MessageDialogOptions {
        title: info.title,
        kind: Some(get_level(&info.kind)),
        buttons: vec![info.ok_label.unwrap_or("OK".to_string()), info.cancel_label.unwrap_or("Cancel".to_string())],
        message: info.message,
        cancel_id: Some(1),
    };
    message(options).await
}

async fn show_ask(info: DialogOptions) -> MessageResult {
    let options = MessageDialogOptions {
        title: info.title,
        kind: Some(get_level(&info.kind)),
        buttons: vec![info.ok_label.unwrap_or("Yes".to_string()), info.cancel_label.unwrap_or("No".to_string()), "Cancel".to_string()],
        message: info.message,
        cancel_id: Some(2),
    };
    message(options).await
}

pub async fn show_file_dialog(option: DialogOptions) -> Option<String> {
    let options = OpenDialogOptions {
        title: option.title,
        default_path: option.default_path,
        filters: None,
        properties: Some(vec![OpenProperty::OpenFile]),
    };

    show_open_dialog(options).await
}

async fn show_open_dialog(options: OpenDialogOptions) -> Option<String> {
    let result = open(options).await;

    if result.canceled {
        None
    } else {
        Some(result.file_paths.first().unwrap().to_string())
    }
}

pub async fn show_save_dialog(option: DialogOptions) -> Option<String> {
    let options = SaveDialogOptions {
        title: option.title,
        default_path: option.default_path,
        filters: None,
    };
    let result = save(options).await;
    if result.canceled {
        None
    } else {
        Some(result.file_paths.first().unwrap().to_string())
    }
}
