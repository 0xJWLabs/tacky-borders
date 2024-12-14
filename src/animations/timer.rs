#![allow(dead_code)]

use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};

use crate::windows_api::SendHWND;
use crate::windows_api::WindowsApi;
use crate::windows_api::WM_APP_TIMER;

/// Enum representing the possible states of the timer.
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum TimerState {
    Running = 0,
    Paused = 1,
    Stopped = 2,
}

/// A timer that can be started, paused, resumed, and stopped.
#[derive(Debug)]
pub struct AnimationTimer {
    state: Arc<(Mutex<TimerState>, Condvar)>,
}

impl AnimationTimer {
    /// Starts a new animation timer that sends messages at the specified interval.
    ///
    /// # Arguments
    ///
    /// * `hwnd` - A handle to the window to send messages to.
    /// * `interval_ms` - The interval (in milliseconds) between timer ticks.
    ///
    /// # Returns
    ///
    /// Returns an `AnimationTimer` instance that can be used to control the timer.
    pub fn start(hwnd: HWND, interval_ms: u64) -> Self {
        let state = Arc::new((Mutex::new(TimerState::Running), Condvar::new()));
        let state_clone = state.clone();
        let window = SendHWND(hwnd);

        thread::spawn(move || {
            let window_sent = window;
            let mut next_tick = Instant::now() + Duration::from_millis(interval_ms);
            loop {
                let (lock, cvar) = &*state_clone;
                let mut state = lock.lock().unwrap();

                // Wait until the timer is not paused
                while *state == TimerState::Paused {
                    state = cvar.wait(state).unwrap(); // Blocks until signaled
                }

                if *state == TimerState::Stopped {
                    break;
                }

                // Send the timer message and schedule next tick
                let now = Instant::now();
                if now >= next_tick {
                    if let Err(e) = WindowsApi::post_message_w(
                        window_sent.0,
                        WM_APP_TIMER,
                        WPARAM(0),
                        LPARAM(0),
                    ) {
                        eprintln!("Could not send animation timer message: {e}");
                        break;
                    }

                    // Schedule next tick
                    next_tick += Duration::from_millis(interval_ms);
                }

                // Sleep until the next tick
                thread::sleep(next_tick.saturating_duration_since(Instant::now()));
            }
        });

        AnimationTimer { state }
    }

    /// Stops the timer, ensuring it no longer sends messages.
    pub fn stop(&mut self) {
        let (lock, cvar) = &*self.state;
        let mut state = lock.lock().unwrap();
        *state = TimerState::Stopped;
        cvar.notify_all(); // Wake up the thread to stop
    }

    /// Pauses the timer, preventing it from sending messages.
    pub fn pause(&mut self) {
        let (lock, cvar) = &*self.state;
        let mut state = lock.lock().unwrap();
        *state = TimerState::Paused;
        cvar.notify_all(); // Wake up the thread to pause
    }

    /// Resumes the timer, allowing it to send messages again.
    pub fn resume(&mut self) {
        let (lock, cvar) = &*self.state;
        let mut state = lock.lock().unwrap();
        *state = TimerState::Running;
        cvar.notify_all(); // Wake up the thread to resume
    }
}
