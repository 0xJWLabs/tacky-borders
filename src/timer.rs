#![allow(non_snake_case)]
use crate::windows_api::SendHWND;
use crate::windows_api::WM_APP_TIMER;
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
    hwnd: Option<SendHWND>,
    timer_id: usize,
    ns_event_id: usize,
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
    timers: Mutex<Vec<TimerHandle>>,
    next_timer_id: Mutex<usize>,
}

impl TimerManager {
    fn next_timer_id(&self) -> usize {
        let mut next_id = self.next_timer_id.lock().unwrap();
        let id = *next_id;
        *next_id += 1;
        id
    }

    fn add_timer(&self, timer_handle: TimerHandle) {
        let mut timers = self.timers.lock().unwrap();

        if let Some(pos) = timers
            .iter()
            .position(|timer| match (&timer.hwnd, &timer_handle.hwnd) {
                (Some(timer_hwnd), Some(handle_hwnd)) => {
                    timer_hwnd.0 == handle_hwnd.0 && timer.ns_event_id == timer_handle.ns_event_id
                }
                (None, Some(_)) | (Some(_), None) | (None, None) => {
                    timer.timer_id == timer_handle.timer_id
                }
            })
        {
            // Stop and replace the existing timer
            timers.remove(pos).stop();
            timers.insert(pos, timer_handle);
        } else {
            timers.push(timer_handle);
        }
    }

    fn remove_timer(&self, hwnd: Option<SendHWND>, u_id_event: usize) {
        let mut timers = self.timers.lock().unwrap();
        if let Some(pos) = timers.iter().position(|timer| match &hwnd {
            Some(handle_hwnd) => {
                timer
                    .hwnd
                    .as_ref()
                    .map_or(false, |timer_hwnd| timer_hwnd.0 == handle_hwnd.0)
                    && timer.ns_event_id == u_id_event
            }
            None => timer.timer_id == u_id_event,
        }) {
            // Stop and remove the timer
            timers.remove(pos).stop();
        } else {
            error!("Timer ID: {} not found", u_id_event);
        }
    }
}

static TIMER_MANAGER: LazyLock<TimerManager> = LazyLock::new(|| TimerManager {
    timers: Mutex::new(Vec::new()),
    next_timer_id: Mutex::new(0),
});

pub unsafe fn SetCustomTimer<P0>(hwnd: P0, ns_event_id: usize, interval_ms: u32) -> usize
where
    P0: Param<HWND>,
{
    // Create a stop flag
    let stop_flag = Arc::new(Mutex::new(false));
    let stop_flag_clone = stop_flag.clone();

    let timer_id = TIMER_MANAGER.next_timer_id();

    let hwnd = Some(hwnd.param().abi());
    let win = hwnd.map(SendHWND);
    let win_clone = win.clone();

    let handle = spawn(move || {
        let interval = Duration::from_millis(interval_ms as u64);

        while !*stop_flag_clone.lock().unwrap() {
            if let Some(win) = win.clone() {
                if PostMessageW(win.0, WM_APP_TIMER, WPARAM(0), LPARAM(0)).is_err() {
                    eprintln!("Error sending message in anim timer");
                    break;
                }
            } else if PostMessageW(None, WM_APP_TIMER, WPARAM(0), LPARAM(0)).is_err() {
                eprintln!("Error sending message in anim timer");
                break;
            }
            sleep(interval);
        }
    });

    // Add the timer handle to the manager
    TIMER_MANAGER.add_timer(TimerHandle {
        stop_flag,
        thread_handle: Some(handle),
        hwnd: win_clone,
        ns_event_id,
        timer_id,
    });

    timer_id
}

pub fn KillCustomTimer<P0>(hwnd: P0, timer_id: usize)
where
    P0: Param<HWND>,
{
    // Convert hwnd to a SendHWND, which is the expected type in remove_timer
    let hwnd = unsafe { Some(hwnd.param().abi()) };
    let win = hwnd.map(SendHWND); // Wrap hwnd in SendHWND

    // Call remove_timer with the correct parameters
    TIMER_MANAGER.remove_timer(win, timer_id);
}
