use super::{take_frame, CaptureError as Error, CapturedFrame, Result};
use crate::{create_capture_item, Handle, WindowRect};
use std::{marker::PhantomData, rc::Rc, time::Duration};
use windows::{
    core::BOOL,
    Win32::{
        Foundation::{
            GetLastError, SetLastError, ERROR_INVALID_WINDOW_HANDLE, ERROR_SUCCESS, HWND, LPARAM,
        },
        System::WinRT::{RoInitialize, RoUninitialize, RO_INIT_MULTITHREADED},
        UI::WindowsAndMessaging::{EnumWindows, GetWindowTextW, IsIconic, IsWindow},
    },
};

struct TitleSearch {
    title: Vec<u16>,
    buffer: Vec<u16>,
    first: Option<HWND>,
    count: usize,
    error: Option<windows::core::Error>,
}
unsafe extern "system" fn match_title(hwnd: HWND, parameter: LPARAM) -> BOOL {
    let search = unsafe { &mut *(parameter.0 as *mut TitleSearch) };
    unsafe { SetLastError(ERROR_SUCCESS) };
    // Extra space prevents accepting a truncated prefix of a longer title.
    let count = unsafe { GetWindowTextW(hwnd, &mut search.buffer) };
    if count == 0 {
        let code = unsafe { GetLastError() };
        if code != ERROR_SUCCESS && code != ERROR_INVALID_WINDOW_HANDLE {
            search.error = Some(windows::core::Error::from_hresult(
                windows::core::HRESULT::from_win32(code.0),
            ));
            return false.into();
        }
    } else if count as usize == search.title.len()
        && search.buffer[..count as usize] == search.title
    {
        search.first.get_or_insert(hwnd);
        search.count += 1;
    }
    true.into()
}

/// Resolve one exact Unicode top-level title; reject duplicate matches.
pub fn find_window_exact(title: &str) -> Result<HWND> {
    if title.is_empty() || title.contains('\0') {
        return Err(Error::InvalidInput(
            "title must be nonempty and contain no NUL",
        ));
    }
    let title: Vec<u16> = title.encode_utf16().collect();
    if title.len() > 32767 {
        return Err(Error::InvalidInput("window title is too long"));
    }
    let mut search = TitleSearch {
        buffer: vec![0; title.len() + 2],
        title,
        first: None,
        count: 0,
        error: None,
    };
    let enumerated = unsafe {
        EnumWindows(
            Some(match_title),
            LPARAM(&mut search as *mut TitleSearch as isize),
        )
    };
    if let Some(error) = search.error {
        return Err(error.into());
    }
    enumerated?;
    match (search.count, search.first) {
        (0, _) => Err(Error::WindowNotFound),
        (1, Some(hwnd)) => Ok(hwnd),
        (count, _) => Err(Error::AmbiguousWindowTitle(count)),
    }
}

fn validate_window(hwnd: HWND) -> Result<()> {
    if hwnd.0.is_null() || !unsafe { IsWindow(Some(hwnd)).as_bool() } {
        return Err(Error::TargetClosed);
    }
    if unsafe { IsIconic(hwnd).as_bool() } {
        return Err(Error::InvalidInput("cannot capture a minimized window"));
    }
    Ok(())
}

/// Capture an HWND without resolving a title. Crop coordinates are frame-relative.
/// The timeout bounds the frame wait, not native driver/encoding operations.
pub fn capture_hwnd(
    hwnd: HWND,
    rect: Option<&WindowRect>,
    timeout: Duration,
) -> Result<CapturedFrame> {
    validate_window(hwnd)?;
    if timeout.is_zero() {
        return Err(Error::InvalidInput("capture timeout must be positive"));
    }
    let _runtime = Runtime::initialize()?;
    let item = create_capture_item(Handle::HWND(hwnd))?;
    take_frame(&item, rect, timeout)
}

// Balance this API's WinRT initialization on the same OS thread, including S_FALSE.
// Legacy initialization remains unchanged in crate::init.
struct Runtime(PhantomData<Rc<()>>);
impl Runtime {
    fn initialize() -> windows::core::Result<Self> {
        unsafe { RoInitialize(RO_INIT_MULTITHREADED)? };
        Ok(Self(PhantomData))
    }
}
impl Drop for Runtime {
    fn drop(&mut self) {
        unsafe { RoUninitialize() }
    }
}
