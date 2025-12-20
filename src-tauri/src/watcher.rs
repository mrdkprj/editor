use crate::helper;
use crossbeam_channel::{bounded, Receiver, Sender};
use notify_debouncer_full::{
    new_debouncer,
    notify::{event::ModifyKind, EventKind, RecursiveMode},
    DebouncedEvent,
};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tauri::{AppHandle, Emitter};

const WATCH_EVENT_NAME: &str = "watch_event";

pub struct WatchTx(pub Sender<WatcherCommand>);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchEvent {
    pub file_path: String,
    pub content: String,
    pub encoding: String,
}

pub enum WatcherCommand {
    Watch(String),
    Unwatch(String),
}

pub fn spwan_watcher(app_handle: &AppHandle, cmd_rx: Receiver<WatcherCommand>) -> Result<(), String> {
    let (tx, rx) = bounded(1);

    let mut watcher = new_debouncer(Duration::from_millis(100), None, move |res| tx.send(res).unwrap_or_default()).map_err(|e| e.to_string())?;
    let app_handle = app_handle.clone();
    tauri::async_runtime::spawn(async move {
        loop {
            crossbeam_channel::select! {
                recv(cmd_rx) -> cmd  => {
                    if let Ok(cmd) = cmd {
                        match cmd {
                            WatcherCommand::Watch(path) => {
                                let _ = watcher.watch(path, RecursiveMode::NonRecursive);
                            },
                            WatcherCommand::Unwatch(path) => {
                                let _ = watcher.unwatch(path);
                            }
                        }
                    }else {
                        println!("Command channel closed. Shutting down watcher.");
                        break;
                    }
                }

                recv(rx) -> event_result => {
                    if let Ok(event_result) = event_result {
                        match event_result {
                            Ok(events) => {
                                for event in events {
                                    if is_modified(event.kind) {
                                       handle_event(&app_handle, &event).unwrap();
                                    }
                                }
                            },
                            Err(errors) => {
                                for error in errors {
                                    eprintln!("[FS_ERR] Watcher error: {:?}", error);
                                }
                            }
                        }
                    }
                }
            }
        }
    });

    Ok(())
}

fn handle_event(app: &AppHandle, event: &DebouncedEvent) -> Result<(), String> {
    let result = helper::read_to_string(event.paths[0].to_str().unwrap()).map_err(|e| notify_debouncer_full::notify::Error::generic(&e)).map_err(|e| e.to_string())?;

    app.emit(
        WATCH_EVENT_NAME,
        WatchEvent {
            file_path: event.paths[0].to_string_lossy().to_string(),
            content: result.content,
            encoding: result.encoding,
        },
    )
    .map_err(|e| e.to_string())
}

fn is_modified(event_kind: EventKind) -> bool {
    matches!(event_kind, EventKind::Modify(ModifyKind::Any) | EventKind::Modify(ModifyKind::Data(_)) | EventKind::Modify(ModifyKind::Metadata(_)) | EventKind::Modify(ModifyKind::Other))
}
