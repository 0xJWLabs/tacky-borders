use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};

use crate::windows_api::SendHWND;
use crate::windows_api::WindowsApi;
use crate::windows_api::WM_APP_TIMER;

#[derive(Debug, Clone)]
pub struct AnimationTimer {
    stop_flag: Arc<AtomicBool>,
}

impl AnimationTimer {
    pub fn start(hwnd: HWND, interval_ms: u64) -> Self {
        let stop_flag = Arc::new(AtomicBool::new(false));
        let stop_flag_clone = stop_flag.clone();

        // Wrap HWND in a struct that implements Send and Sync to move it into the thread
        let window = SendHWND(hwnd);

        // Spawn a worker thread for the timer
        thread::spawn(move || {
            let window_sent = window;
            let interval = Duration::from_millis(interval_ms);

            while !stop_flag_clone.load(Ordering::SeqCst) {
                if let Err(e) =
                    WindowsApi::post_message_w(window_sent.0, WM_APP_TIMER, WPARAM(0), LPARAM(0))
                {
                    error!("could not send animation timer message: {e}");
                    break;
                }
                thread::sleep(interval);
            }
        });

        // Return the timer instance
        Self { stop_flag }
    }

    pub fn stop(&mut self) {
        self.stop_flag.store(true, Ordering::SeqCst);
    }
}
