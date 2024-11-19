#![allow(non_snake_case)]
use crate::windowsapi::SendHWND;
use crate::windowsapi::WM_APP_TIMER;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::LazyLock;
use std::sync::Mutex;
use std::thread::sleep;
use std::thread::spawn;
use std::thread::JoinHandle;
use std::time::Duration;
use windows::core::Param;
use windows::Win32::Foundation::HWND;
use windows::Win32::Foundation::LPARAM;
use windows::Win32::Foundation::WPARAM;
use windows::Win32::UI::WindowsAndMessaging::PostMessageW;

#[derive(Debug)]
pub struct TimerHandle {
    stop_flag: Arc<Mutex<bool>>,
    thread_handle: Option<JoinHandle<()>>,
}

impl TimerHandle {
    pub fn stop(&mut self) {
        // Signal the worker thread to stop
        if let Ok(mut flag) = self.stop_flag.lock() {
            *flag = true;
        }

        // Wait for the worker thread to finish
        if let Some(handle) = self.thread_handle.take() {
            handle.join().unwrap();
        }
    }
}

// Timer manager to store and retrieve timers by ID
#[derive(Debug)]
pub struct TimerManager {
    timers: Mutex<HashMap<usize, TimerHandle>>,
    next_timer_id: Mutex<usize>,
}

impl TimerManager {
    fn next_timer_id(&self) -> usize {
        let mut next_id = self.next_timer_id.lock().unwrap();
        let id = *next_id;
        *next_id += 1;
        id
    }

    fn add_timer(&self, timer_id: usize, timer_handle: TimerHandle) {
        let mut timers = self.timers.lock().unwrap();
        timers.insert(timer_id, timer_handle);
    }

    fn remove_timer(&self, timer_id: usize) {
        let mut timers = self.timers.lock().unwrap();
        if let Some(mut timer) = timers.remove(&timer_id) {
            timer.stop();
        } else {
            eprintln!("Error: Timer ID {} not found", timer_id);
        }
    }
}

static TIMER_MANAGER: LazyLock<TimerManager> = LazyLock::new(|| TimerManager {
    timers: Mutex::new(HashMap::new()),
    next_timer_id: Mutex::new(0),
});

pub unsafe fn SetCustomTimer<P0>(hwnd: P0, interval_ms: u32) -> usize
where
    P0: Param<HWND>,
{
    // Create a stop flag
    let stop_flag = Arc::new(Mutex::new(false));
    let stop_flag_clone = stop_flag.clone();

    let timer_id = TIMER_MANAGER.next_timer_id();

    // Wrap HWND in a struct to move it into the thread safely
    let win = SendHWND(hwnd.param().abi());

    // Spawn a worker thread for the timer
    let handle = spawn(move || {
        let window = win;
        let interval = Duration::from_millis(interval_ms as u64);
        while !*stop_flag_clone.lock().unwrap() {
            if PostMessageW(window.0, WM_APP_TIMER, WPARAM(0), LPARAM(0)).is_err() {
                eprintln!("Error sending message in anim timer");
                break;
            }
            sleep(interval);
        }
    });

    // Return the timer handle

    TIMER_MANAGER.add_timer(
        timer_id,
        TimerHandle {
            stop_flag,
            thread_handle: Some(handle),
        },
    );

    timer_id
}

pub fn KillCustomTimer(timer_id: usize) {
    TIMER_MANAGER.remove_timer(timer_id);
}
