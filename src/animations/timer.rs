#![allow(dead_code)]

use crate::utils::LogIfErr;
use crate::window_border::WindowBorder;
use crate::windows_api::WM_APP_TIMER;
use crate::windows_api::{SendHWND, WindowsApi};
use anyhow::{anyhow, Result as AnyResult};
use rustc_hash::FxHashMap;
use std::collections::hash_map::Entry;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock, Mutex, RwLock};
use std::thread::{sleep, spawn, JoinHandle};
use std::time::{Duration, Instant};
use windows::Win32::Foundation::{LPARAM, WPARAM};

/// Global animation timer manager.
/// Manages and coordinates timers for animated window borders.
pub static TIMER_MANAGER: LazyLock<Arc<RwLock<GlobalAnimationTimer>>> =
    LazyLock::new(|| Arc::new(RwLock::new(GlobalAnimationTimer::new())));

/// A manager for animation timers, ensuring timers are associated with specific windows.
pub struct GlobalAnimationTimer {
    /// Map of timers keyed by window handle (as `usize`).
    timers: Arc<RwLock<FxHashMap<usize, AnimationTimer>>>,
}

impl GlobalAnimationTimer {
    /// Creates a new instance of the `GlobalAnimationTimer` with an empty set of timers.
    ///
    /// # Returns
    /// * A new instance of `GlobalAnimationTimer`.
    pub fn new() -> Self {
        Self {
            timers: Arc::new(RwLock::new(FxHashMap::default())),
        }
    }

    /// Adds a new timer for a specific window.
    ///
    /// # Arguments
    /// * `border` - A reference to the `WindowBorder`.
    /// * `timer` - The `AnimationTimer` to be added.
    ///
    /// # Returns
    /// * `Ok(())` if the timer was added successfully.
    /// * `Err` if a timer for the window already exists.
    pub fn add_timer(&self, border: &WindowBorder, timer: AnimationTimer) -> AnyResult<()> {
        let mut timers = self.timers.write().unwrap(); // Lock for writing
        if let Entry::Vacant(e) = timers.entry(border.border_window.0 as usize) {
            e.insert(timer.clone());
            Ok(())
        } else {
            Err(anyhow!("timer with this hwnd already exists."))
        }
    }

    /// Removes the timer associated with a specific window.
    ///
    /// # Arguments
    /// * `border` - A reference to the `WindowBorder`.
    ///
    /// # Returns
    /// * `Ok(())` if the timer was removed successfully.
    /// * `Err` if no timer was found for the specified window.
    pub fn remove_timer(&self, border: &WindowBorder) -> AnyResult<()> {
        let border_u = border.border_window.0 as usize;
        let mut timers = self.timers.write().unwrap(); // Lock for writing

        if let Entry::Occupied(e) = timers.entry(border_u) {
            let timer = e.get();
            if timer.0.load(Ordering::SeqCst) {
                timer.0.store(false, Ordering::SeqCst);
            }

            if let Some(handle) = timer.1.lock().unwrap().take() {
                handle
                    .join()
                    .map_err(|e| anyhow!("failed to join timer thread: {e:?}"))?;
            }

            e.remove();
            Ok(())
        } else {
            Err(anyhow!("no matching timer found for the provided HWND"))
        }
    }
}

/// A timer that sends messages at a specified interval to animate window borders.
#[derive(Debug, Clone)]
pub struct AnimationTimer(Arc<AtomicBool>, Arc<Mutex<Option<JoinHandle<()>>>>);

impl AnimationTimer {
    /// Starts a new animation timer for a window.
    ///
    /// # Arguments
    /// * `border` - The `WindowBorder` to associate the timer with.
    /// * `interval_ms` - The interval in milliseconds between timer ticks.
    ///
    /// # Returns
    /// * A `Result` containing the `AnimationTimer` on success, or an error otherwise.
    pub fn start(border: &mut WindowBorder, interval_ms: u64) -> AnyResult<AnimationTimer> {
        let running = Arc::new(AtomicBool::new(true));
        let running_clone = running.clone();
        let window_sent = SendHWND(border.border_window);
        let handle = spawn(move || {
            let window_sent = window_sent;
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
                sleep(next_tick.saturating_duration_since(Instant::now()));
            }
        });

        let timer = Self(running, Arc::new(Mutex::new(Some(handle))));

        TIMER_MANAGER
            .write()
            .unwrap()
            .add_timer(&border.clone(), timer.clone())
            .log_if_err();

        border.animations.timer = Some(timer.clone());
        border.last_animation_time = Some(Instant::now());

        Ok(timer)
    }

    /// Stops the timer of a window from sending further messages.
    ///
    /// # Returns
    /// * `Ok(())` if the timer was stopped successfully.
    pub fn stop(border: &mut WindowBorder) -> AnyResult<()> {
        TIMER_MANAGER
            .write()
            .unwrap()
            .remove_timer(border)
            .log_if_err();
        border.animations.timer = None;
        Ok(())
    }
}

#[allow(non_snake_case)]
/// Sets an animation timer for the provided `WindowBorder` if needed.
///
/// This function checks an optional condition, and if the condition is met or not provided,
/// it starts the animation timer if one is not already running.
///
/// # Arguments
/// * `border` - A mutable reference to the `WindowBorder` to set the timer for.
/// * `condition` - An optional condition function that must return `true` for the timer to be set.
///
/// # Returns
/// * `Ok(())` if the timer was set successfully, or an error if the timer could not be started.
pub fn SetAnimationTimer<F>(border: &mut WindowBorder, condition: Option<F>) -> AnyResult<()>
where
    F: Fn(&WindowBorder) -> bool,
{
    // If condition exists, check it; otherwise, proceed directly
    if condition.map_or(true, |cond| cond(border)) && border.animations.timer.is_none() {
        let timer_duration = (1000.0 / border.animations.fps as f32) as u64;
        AnimationTimer::start(border, timer_duration).log_if_err();
    }
    Ok(())
}

#[allow(non_snake_case)]
/// Kills the animation timer for the provided `WindowBorder`.
///
/// This function stops and removes the animation timer for the specified window border.
///
/// # Arguments
/// * `border` - A mutable reference to the `WindowBorder` to remove the timer from.
///
/// # Returns
/// * `Ok(())` if the timer was successfully killed, or an error if stopping the timer failed.
pub fn KillAnimationTimer(border: &mut WindowBorder) -> AnyResult<()> {
    if border.animations.timer.is_some() {
        AnimationTimer::stop(border).log_if_err();
    }
    Ok(())
}
