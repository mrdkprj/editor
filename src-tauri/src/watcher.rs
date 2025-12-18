#![allow(unused_imports)]
use crate::helper;
use notify_debouncer_full::{
    new_debouncer,
    notify::{event::ModifyKind, EventKind, ReadDirectoryChangesWatcher, RecursiveMode},
    DebounceEventResult, DebouncedEvent, Debouncer, FileIdMap,
};
use once_cell::sync::{Lazy, OnceCell};
use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    time::Duration,
};
use tauri::{AppHandle, Emitter as _, EventTarget, WebviewWindow};
use tokio::sync::{
    mpsc::{self, UnboundedReceiver, UnboundedSender},
    oneshot,
};

const WATCH_EVENT_NAME: &str = "watch_event";

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

pub fn spwan_watcher(app_handle: &AppHandle, mut cmd_rx: UnboundedReceiver<WatcherCommand>) -> Result<(), String> {
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

    let mut watcher = new_debouncer(Duration::from_millis(100), None, move |res| tx.send(res).unwrap_or_default()).map_err(|e| e.to_string())?;
    let app_handle = app_handle.clone();
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::select! {
                cmd = cmd_rx.recv() => {
                    match cmd {
                        Some(WatcherCommand::Watch(path)) => {
                            let _ = watcher.watch(path, RecursiveMode::NonRecursive);
                        },
                        Some(WatcherCommand::Unwatch(path)) => {
                            let _ = watcher.unwatch(path);
                        },
                        None => {
                            println!("Command channel closed. Shutting down watcher.");
                            break;
                        }
                    }
                }

                event_result = rx.recv() => {
                    if let Some(event_result) = event_result {
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
