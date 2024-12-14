use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};

use crate::windows_api::SendHWND;
use crate::windows_api::WindowsApi;
use crate::windows_api::WM_APP_TIMER;

#[derive(Debug)]
pub struct AnimationTimer {
    running: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

impl AnimationTimer {
    pub fn start(hwnd: HWND, interval_ms: u64) -> Self {
        let running = Arc::new(AtomicBool::new(true));
        let running_clone = running.clone();

        // Wrap HWND in a struct that implements Send and Sync to move it into the thread
        let window = SendHWND(hwnd);

        // Spawn a worker thread for the timer
        let worker = thread::spawn(move || {
            let window_sent = window;
            let interval = Duration::from_millis(interval_ms);

            while running_clone.load(Ordering::SeqCst) {
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
        Self {
            running,
            worker: Some(worker),
        }
    }

    pub fn stop(&mut self) {
        self.running.store(false, Ordering::SeqCst);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

impl Drop for AnimationTimer {
    fn drop(&mut self) {
        self.stop(); // Ensure the thread stops on drop
    }
}
