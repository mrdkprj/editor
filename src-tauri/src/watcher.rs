use async_std::{
    channel::{self, Receiver},
    sync::Mutex,
    task::block_on,
};
use notify_debouncer_full::{
    new_debouncer,
    notify::{event::ModifyKind, EventKind, ReadDirectoryChangesWatcher, RecursiveMode},
    DebounceEventResult, DebouncedEvent, Debouncer, FileIdMap,
};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::path::Path;
use tauri::{Emitter, EventTarget, WebviewWindow};

use crate::helper;

static WATCHER: Lazy<Mutex<Option<Debouncer<ReadDirectoryChangesWatcher, FileIdMap>>>> = Lazy::new(|| Mutex::new(None));
static HANDLED: Lazy<Mutex<bool>> = Lazy::new(|| Mutex::new(false));

const WATCH_EVENT_NAME: &str = "watch_event";

fn async_watcher() -> notify_debouncer_full::notify::Result<(Debouncer<ReadDirectoryChangesWatcher, FileIdMap>, Receiver<DebouncedEvent>)> {
    let (tx, rx) = channel::bounded(1);

    let watcher = new_debouncer(std::time::Duration::from_millis(50), None, move |result: DebounceEventResult| match result {
        Ok(events) => events.iter().for_each(|event| {
            block_on(async {
                let _ = tx.send(event.clone()).await;
            })
        }),
        Err(errors) => errors.iter().for_each(|error| println!("{error:?}")),
    })?;

    Ok((watcher, rx))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WatchEvent {
    file_path: String,
    content: String,
    encoding: String,
}

pub async fn watch(window: &WebviewWindow, file_path: String) -> notify_debouncer_full::notify::Result<()> {
    let (mut watcher, rx) = async_watcher()?;

    // Add a path to be watched. All files and directories at that path and
    // below will be monitored for changes.
    watcher.watch(Path::new(&file_path), RecursiveMode::NonRecursive)?;

    {
        let mut inner = WATCHER.try_lock().unwrap();
        *inner = Some(watcher);
    }

    while let Ok(event) = rx.recv().await {
        if is_modified(event.kind) {
            let mut handled = HANDLED.try_lock().unwrap();

            let result = helper::read_to_string(event.paths[0].to_str().unwrap()).map_err(|e| notify_debouncer_full::notify::Error::generic(&e))?;

            window
                .emit_to(
                    EventTarget::WebviewWindow {
                        label: window.label().to_string(),
                    },
                    WATCH_EVENT_NAME,
                    WatchEvent {
                        file_path: event.paths[0].to_string_lossy().to_string(),
                        content: result.content,
                        encoding: result.encoding,
                    },
                )
                .unwrap();
            *handled = false;
        }
    }

    Ok(())
}

fn is_modified(event_kind: EventKind) -> bool {
    matches!(event_kind, EventKind::Modify(ModifyKind::Any) | EventKind::Modify(ModifyKind::Data(_)) | EventKind::Modify(ModifyKind::Metadata(_)) | EventKind::Modify(ModifyKind::Other))
}

pub fn unwatch(file_path: String) {
    if let Some(mut inner) = WATCHER.try_lock() {
        if let Some(watcher) = inner.as_mut() {
            let _ = watcher.unwatch(Path::new(&file_path));
        }
    }
}
