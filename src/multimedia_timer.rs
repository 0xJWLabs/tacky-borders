use crate::windowsapi::WM_APP_TIMER;
use windows::Win32::Foundation::HWND;
use windows::Win32::Foundation::LPARAM;
use windows::Win32::Foundation::WPARAM;
use windows::Win32::Media::timeBeginPeriod;
use windows::Win32::Media::timeEndPeriod;
use windows::Win32::Media::timeKillEvent;
use windows::Win32::Media::timeSetEvent;
use windows::Win32::Media::TIME_PERIODIC;
use windows::Win32::UI::WindowsAndMessaging::PostMessageW;

#[derive(Debug, Clone)]
pub struct MultimediaTimer {
    pub timer_id: Option<u32>,
}

impl MultimediaTimer {
    pub fn start(hwnd: HWND, interval_ms: u32) -> Self {
        unsafe {
            timeBeginPeriod(1);
            let timer_id = timeSetEvent(
                interval_ms,
                1,
                Some(timer_callback),
                hwnd.0 as usize,
                TIME_PERIODIC,
            );
            Self {
                timer_id: if timer_id != 0 { Some(timer_id) } else { None },
            }
        }
    }

    pub fn stop(&mut self) {
        if let Some(timer_id) = self.timer_id {
            unsafe {
                timeKillEvent(timer_id);
                timeEndPeriod(1);
            }
            self.timer_id = None;
        }
    }
}

unsafe extern "system" fn timer_callback(_: u32, _: u32, user_data: usize, _: usize, _: usize) {
    let hwnd = HWND(user_data as _);
    let _ = PostMessageW(hwnd, WM_APP_TIMER, WPARAM(0), LPARAM(0)); // Custom message ID 0x8001
}
