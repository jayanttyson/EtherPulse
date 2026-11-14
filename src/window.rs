use std::ffi::c_void;
use std::os::windows::process::CommandExt;
use std::process::Command;

#[link(name = "kernel32")]
#[link(name = "shell32")]
extern "system" {
    fn CreateMutexW(
        lpMutexAttributes: *const c_void,
        bInitialOwner: i32,
        lpName: *const u16,
    ) -> *mut c_void;
    fn GetLastError() -> u32;

    fn ShellExecuteW(
        hwnd: *mut c_void,
        operation: *const u16,
        file: *const u16,
        parameters: *const u16,
        directory: *const u16,
        show_cmd: i32,
    ) -> *mut c_void;
}

const ERROR_ALREADY_EXISTS: u32 = 183;
const CREATE_NO_WINDOW: u32 = 0x08000000;

pub fn is_already_running() -> bool {
    let name: Vec<u16> = "Global\\LanCableLoggerSingleInstanceMutex"
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();

    unsafe {
        let handle = CreateMutexW(std::ptr::null(), 0, name.as_ptr());
        if !handle.is_null() && GetLastError() == ERROR_ALREADY_EXISTS {
            return true;
        }
    }
    false
}

pub fn launch_desktop_window(url: &str) {
    let browser_candidates = [
        r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
        r"C:\Program Files\Microsoft\Edge\Application\msedge.exe",
        r"C:\Program Files\Google\Chrome\Application\chrome.exe",
        r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe",
    ];

    let profile_dir = std::env::var("LOCALAPPDATA")
        .map(|p| std::path::PathBuf::from(p).join("LanCableLogger").join("profile"))
        .unwrap_or_else(|_| std::env::temp_dir().join("LanCableLoggerProfile"));

    let _ = std::fs::create_dir_all(&profile_dir);

    for browser in &browser_candidates {
        if std::path::Path::new(browser).exists() {
            let res = Command::new(browser)
                .arg(format!("--app={}", url))
                .arg(format!("--user-data-dir={}", profile_dir.display()))
                .arg("--no-first-run")
                .arg("--no-default-browser-check")
                .arg("--window-size=1160,780")
                .creation_flags(CREATE_NO_WINDOW)
                .spawn();

            if res.is_ok() {
                return;
            }
        }
    }

    // Pure Rust Win32 fallback via ShellExecuteW (ZERO cmd.exe)
    open_url_native(url);
}

pub fn open_url_native(url: &str) {
    let operation: Vec<u16> = "open\0".encode_utf16().collect();
    let target: Vec<u16> = format!("{}\0", url).encode_utf16().collect();
    unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            operation.as_ptr(),
            target.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            1, // SW_SHOWNORMAL
        );
    }
}

pub fn open_file_native(file_path: &str) {
    let operation: Vec<u16> = "open\0".encode_utf16().collect();
    let file: Vec<u16> = format!("{}\0", file_path).encode_utf16().collect();
    unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            operation.as_ptr(),
            file.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            1, // SW_SHOWNORMAL
        );
    }
}
