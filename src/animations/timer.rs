#![allow(dead_code)]

use anyhow::{anyhow, Result as AnyResult};
use rustc_hash::FxHashMap;
use std::collections::hash_map::Entry;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock, RwLock};
use std::thread;
use std::time::{Duration, Instant};
use windows::core::Param;
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};

use crate::utils::LogIfErr;
use crate::window_border::WindowBorder;
use crate::windows_api::SendHWND;
use crate::windows_api::WindowsApi;
use crate::windows_api::WM_APP_TIMER;

pub static TIMER_MANAGER: LazyLock<Arc<RwLock<GlobalAnimationTimer>>> =
    LazyLock::new(|| Arc::new(RwLock::new(GlobalAnimationTimer::new())));

pub struct GlobalAnimationTimer {
    timers: Arc<RwLock<FxHashMap<usize, AnimationTimer>>>,
}

impl GlobalAnimationTimer {
    /// Creates a new instance of the `GlobalAnimationTimer` with an empty set of timers.
    pub fn new() -> Self {
        Self {
            timers: Arc::new(RwLock::new(FxHashMap::default())),
        }
    }

    /// Adds a new timer for the specified window.
    ///
    /// # Arguments
    /// * `hwnd` - A handle to the window the timer is associated with.
    /// * `timer` - The `AnimationTimer` to be added.
    ///
    /// # Returns
    /// * `Ok(())` if the timer was added successfully.
    /// * `Err` if the timer already exists for the specified window handle.
    pub fn add_timer<P0>(&self, hwnd: P0, timer: AnimationTimer) -> AnyResult<()>
    where
        P0: Param<HWND>,
    {
        let hwnd = unsafe { hwnd.param().abi() };

        let mut timers = self.timers.write().unwrap(); // Lock for writing
        if let Entry::Vacant(e) = timers.entry(hwnd.0 as usize) {
            e.insert(timer.clone());
            Ok(())
        } else {
            Err(anyhow!("timer with this hwnd already exists."))
        }
    }

    /// Removes the timer for the specified window handle.
    ///
    /// # Arguments
    /// * `hwnd` - A handle to the window associated with the timer.
    ///
    /// # Returns
    /// * `Ok(())` if the timer was removed successfully.
    /// * `Err` if no timer was found for the specified window handle.
    pub fn remove_timer<P0>(&self, hwnd: P0) -> AnyResult<()>
    where
        P0: Param<HWND>,
    {
        let hwnd = unsafe { hwnd.param().abi() };
        let hwnd_u = hwnd.0 as usize;
        if let Some(timer) = self.get_timer(hwnd_u) {
            let mut timers = self.timers.write().unwrap(); // Lock for writing
            timer.stop().log_if_err();
            timers.remove(&hwnd_u);
            Ok(())
        } else {
            Err(anyhow!("No matching timer found for the provided HWND"))
        }
    }

    /// Fetches the timer associated with the specified window handle.
    ///
    /// # Arguments
    /// * `hwnd_u` - The window handle as a `usize`.
    ///
    /// # Returns
    /// * `Some(timer)` if the timer exists for the given window handle.
    /// * `None` if no timer was found for the window handle.
    pub fn get_timer(&self, hwnd_u: usize) -> Option<AnimationTimer> {
        let timers = self.timers.read().unwrap(); // Lock for reading
        timers.get(&hwnd_u).cloned()
    }
}

/// A timer that sends messages at a specified interval.
#[derive(Debug, Clone)]
pub struct AnimationTimer {
    running: Arc<AtomicBool>,
}

impl AnimationTimer {
    /// Starts a new animation timer for the specified window handle.
    ///
    /// # Arguments
    /// * `hwnd` - The window handle to associate the timer with.
    /// * `interval_ms` - The interval between timer ticks, in milliseconds.
    ///
    /// # Returns
    /// * A `Result` containing the `AnimationTimer` on success, or an error on failure.
    pub fn start(hwnd: HWND, interval_ms: u64) -> AnyResult<AnimationTimer> {
        let running = Arc::new(AtomicBool::new(true));
        let running_clone = running.clone();
        let window = SendHWND(hwnd);
        thread::spawn(move || {
            let window_sent = window;
            let mut next_tick = Instant::now() + Duration::from_millis(interval_ms);
            while running_clone.load(Ordering::SeqCst) {
                // Send the timer message and schedule next tick
                if Instant::now() >= next_tick {
                    if let Err(e) = WindowsApi::post_message_w(
                        window_sent.0,
                        WM_APP_TIMER,
                        WPARAM(0),
                        LPARAM(0),
                    ) {
                        error!("could not send animation timer message: {e}");
                        break;
                    }

                    // Schedule next tick
                    next_tick += Duration::from_millis(interval_ms);
                }

                // Sleep until the next tick
                thread::sleep(next_tick.saturating_duration_since(Instant::now()));
            }
        });

        let timer = AnimationTimer { running };
        TIMER_MANAGER
            .write()
            .unwrap()
            .add_timer(hwnd, timer.clone())
            .log_if_err();

        Ok(timer)
    }

    /// Stops the timer from sending further messages.
    ///
    /// # Returns
    /// * `Ok(())` if the timer was stopped successfully.
    pub fn stop(&self) -> AnyResult<()> {
        self.running.store(false, Ordering::SeqCst);
        Ok(())
    }
}

#[allow(non_snake_case)]
/// Sets an animation timer for the provided `WindowBorder` if needed.
///
/// # Arguments
/// * `border` - The mutable reference to the `WindowBorder` to set the timer for.
///
/// # Returns
/// * `Ok(())` if the timer was set successfully, or an error if the conditions are not met.
pub fn SetAnimationTimer(border: &mut WindowBorder) -> AnyResult<()> {
    if (!border.animations.active.is_empty() || !border.animations.inactive.is_empty())
        && border.animations.timer.is_none()
    {
        let timer_duration = (1000.0 / border.animations.fps as f32) as u64;
        border.animations.timer =
            Some(AnimationTimer::start(border.border_window, timer_duration)?);

        border.last_animation_time = Some(Instant::now());
    }
    Ok(())
}

#[allow(non_snake_case)]
/// Kills the animation timer for the provided `WindowBorder`.
///
/// # Arguments
/// * `border` - The mutable reference to the `WindowBorder` to remove the timer from.
///
/// # Returns
/// * `Ok(())` if the timer was successfully killed.
pub fn KillAnimationTimer(border: &mut WindowBorder) -> AnyResult<()> {
    if border.animations.timer.is_some() {
        TIMER_MANAGER
            .write()
            .unwrap()
            .remove_timer(border.border_window)
            .log_if_err();
        border.animations.timer = None;
    }
    Ok(())
}
