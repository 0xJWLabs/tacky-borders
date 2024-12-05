#![allow(non_snake_case)]
use crate::windows_api::SendHWND;
use crate::windows_api::WindowsApi;
use crate::windows_api::WM_APP_TIMER;
use rustc_hash::FxHashMap;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
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

#[derive(Debug)]
pub struct TimerHandle {
    stop_flag: Arc<AtomicBool>,
    thread_handle: Option<JoinHandle<()>>,
    hwnd: Option<SendHWND>,
    ns_event_id: usize,
}

impl TimerHandle {
    pub fn stop(&mut self) {
        // Set the stop flag to true atomically
        self.stop_flag.store(true, Ordering::SeqCst);

        // Attempt to join the thread and log any errors
        if let Some(handle) = self.thread_handle.take() {
            if let Err(e) = handle.join() {
                error!("Failed to join timer thread: {:?}", e);
            }
        }
    }
}

// Timer manager to store and retrieve timers by ID
#[derive(Debug)]
pub struct TimerManager {
    timers: Mutex<FxHashMap<usize, TimerHandle>>,
    next_timer_id: Mutex<usize>,
}

impl TimerManager {
    fn next_timer_id(&self) -> usize {
        let mut next_id = self.next_timer_id.lock().unwrap();
        let id = *next_id;
        *next_id += 1;
        id
    }

    fn find_timer(&self, hwnd: Option<&SendHWND>, ns_event_id: usize) -> Option<usize> {
        let timers = self.timers.lock().unwrap();

        if let Some(hwnd) = hwnd {
            // If hwnd is provided, find a timer by matching ns_event_id and hwnd
            timers.iter().find_map(|(&timer_id, timer)| {
                if timer.ns_event_id == ns_event_id
                    && timer.hwnd.as_ref().map_or(false, |th| th.0 == hwnd.0)
                {
                    Some(timer_id)
                } else {
                    None
                }
            })
        } else {
            // If hwnd is None, treat ns_event_id as the timer_id (key)
            if timers.contains_key(&ns_event_id) {
                Some(ns_event_id)
            } else {
                None
            }
        }
    }

    fn add_timer(&self, timer_handle: TimerHandle, next_timer_id: usize) {
        let timer_id = match timer_handle.hwnd.as_ref() {
            Some(hwnd) => self.find_timer(Some(hwnd), timer_handle.ns_event_id),
            None => self.find_timer(None, next_timer_id),
        };

        let mut timers = self.timers.lock().unwrap();

        match timer_id {
            Some(id) => {
                if let Some(existing_timer) = timers.get_mut(&id) {
                    existing_timer.stop();
                    *existing_timer = timer_handle;
                }
            }
            None => {
                timers.insert(next_timer_id, timer_handle);
            }
        };
    }

    fn remove_timer(&self, hwnd: Option<SendHWND>, u_event_id: usize) {
        let mut timers = self.timers.lock().unwrap();
        let timer_id = match hwnd.as_ref() {
            Some(hwnd) => self.find_timer(Some(hwnd), u_event_id),
            None => self.find_timer(None, u_event_id),
        };

        if let Some(id) = timer_id {
            if let Some(timer) = timers.get_mut(&id) {
                timer.stop();
            }
            timers.remove(&id);
        } else {
            error!("Timer with event id {u_event_id} not found");
        }
    }
}

static TIMER_MANAGER: LazyLock<TimerManager> = LazyLock::new(|| TimerManager {
    timers: Mutex::new(FxHashMap::default()),
    next_timer_id: Mutex::new(0),
});

pub unsafe fn SetCustomTimer<P0>(hwnd: P0, ns_event_id: usize, interval_ms: u32) -> usize
where
    P0: Param<HWND>,
{
    // Create a stop flag
    let stop_flag = Arc::new(AtomicBool::new(false));
    let stop_flag_clone = stop_flag.clone();

    let timer_id = TIMER_MANAGER.next_timer_id();

    let hwnd = Some(hwnd.param().abi());
    let win = hwnd.map(SendHWND);
    let win_clone = win.clone();

    let handle = spawn(move || {
        let stop_f = stop_flag_clone;
        let window_sent = win;
        let interval = Duration::from_millis(interval_ms as u64);

        while !stop_f.load(Ordering::SeqCst) {
            if let Some(win) = window_sent.clone() {
                if let Err(e) =
                    WindowsApi::post_message_w(win.0, WM_APP_TIMER, WPARAM(0), LPARAM(0))
                {
                    error!("could not send animation timer message: {e}");
                    break;
                }
            } else if let Err(e) =
                WindowsApi::post_message_w(None, WM_APP_TIMER, WPARAM(0), LPARAM(0))
            {
                error!("could not send animation timer message: {e}");

                break;
            }
            sleep(interval);
        }
    });

    // Add the timer handle to the manager
    TIMER_MANAGER.add_timer(
        TimerHandle {
            stop_flag,
            thread_handle: Some(handle),
            hwnd: win_clone,
            ns_event_id,
        },
        timer_id,
    );

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
