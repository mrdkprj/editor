#[cfg(target_os = "windows")]
use windows::{
    core::PCWSTR,
    Win32::{
        Foundation::{CloseHandle, GetLastError, ERROR_ALREADY_EXISTS, HANDLE},
        System::Threading::{CreateMutexW, ReleaseMutex},
    },
};
#[cfg(target_os = "linux")]
use zbus::blocking::{connection::Builder, Connection};

pub struct Session {
    #[cfg(target_os = "linux")]
    pub connection: Connection,
    #[cfg(target_os = "windows")]
    pub mutex: isize,
    #[cfg(target_os = "linux")]
    pub id: String,
}

#[cfg(target_os = "windows")]
fn encode_wide(string: impl AsRef<std::ffi::OsStr>) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    string.as_ref().encode_wide().chain(std::iter::once(0)).collect()
}

pub fn start(id: &str) -> Result<Session, String> {
    let id = if cfg!(target_os = "linux") {
        format!("org.{}.session", id.replace(['.', '-'], "_"))
    } else {
        id.to_string()
    };
    #[cfg(target_os = "windows")]
    unsafe {
        let mutex_name = encode_wide(&id);
        let mutex = CreateMutexW(None, true, PCWSTR::from_raw(mutex_name.as_ptr())).map_err(|e| e.message())?;
        if GetLastError() == ERROR_ALREADY_EXISTS {
            Err("Cannot start session".to_string())
        } else {
            Ok(Session {
                mutex: mutex.0 as isize,
            })
        }
    }

    #[cfg(target_os = "linux")]
    {
        if let Ok(connection) = Builder::session().unwrap().name(&id).unwrap().replace_existing_names(false).allow_name_replacements(false).build() {
            Ok(Session {
                connection,
                id,
            })
        } else {
            Err("Cannot start session".to_string())
        }
    }
}

pub fn end(session: &Session) {
    #[cfg(target_os = "windows")]
    unsafe {
        let _ = ReleaseMutex(HANDLE(session.mutex as _));
        let _ = CloseHandle(HANDLE(session.mutex as _));
    }

    #[cfg(target_os = "linux")]
    let _ = session.connection.release_name(&session.id);
}
