#![allow(dead_code)]

use crate::as_ptr;
use crate::border_manager::Border;
use crate::error::LogIfErr;
use crate::windows_api::WindowsApi;
use crate::windows_api::WM_APP_TIMER;
use anyhow::{anyhow, Result as AnyResult};
use rustc_hash::FxHashMap;
use std::collections::hash_map::Entry;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock, Mutex};
use std::thread::{sleep, spawn};
use std::time::{Duration, Instant};
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};

const NUM_SHARDS: usize = 16;

/// Global animation timer manager.
/// Manages and coordinates timers for animated window borders.
pub static TIMER_MANAGER: LazyLock<Arc<Mutex<GlobalAnimationTimer>>> =
    LazyLock::new(|| Arc::new(Mutex::new(GlobalAnimationTimer::new())));

/// A manager for animation timers, ensuring timers are associated with specific windows.
#[derive(Debug)]
pub struct GlobalAnimationTimer {
    /// Map of timers keyed by window handle (as `usize`).
    timers: Vec<Arc<Mutex<FxHashMap<usize, AnimationTimer>>>>,
}

impl GlobalAnimationTimer {
    /// Creates a new instance of the `GlobalAnimationTimer` with an empty set of timers.
    ///
    /// # Returns
    /// * A new instance of `GlobalAnimationTimer`.
    pub fn new() -> Self {
        let mut timers = Vec::with_capacity(NUM_SHARDS);
        for _ in 0..NUM_SHARDS {
            timers.push(Arc::new(Mutex::new(FxHashMap::default())));
        }

        Self { timers }
    }

    /// Gets the shard index based on the window handle.
    ///
    /// # Returns
    /// * Index of shard
    fn get_shard_index(&self, hwnd: usize) -> usize {
        hwnd % NUM_SHARDS
    }

    /// Adds a new timer for a specific window.
    ///
    /// # Arguments
    /// * `border` - A reference to the `Border`.
    /// * `timer` - The `AnimationTimer` to be added.
    ///
    /// # Returns
    /// * `Ok(())` if the timer was added successfully.
    /// * `Err` if a timer for the window already exists.
    pub fn add_timer(&self, border: &Border, timer: AnimationTimer) -> AnyResult<()> {
        let hwnd_u = border.border_window as usize;
        let shard_index = self.get_shard_index(hwnd_u);
        // Attempt to acquire the lock safely
        let mut timers = self.timers[shard_index]
            .lock()
            .map_err(|e| anyhow!("failed to acquire lock for timers: {e}"))?;

        if let Entry::Vacant(e) = timers.entry(hwnd_u) {
            e.insert(timer.clone());
            Ok(())
        } else {
            Err(anyhow!("timer with this hwnd already exists."))
        }
    }

    /// Removes the timer associated with a specific window.
    ///
    /// # Arguments
    /// * `border` - A reference to the `Border`.
    ///
    /// # Returns
    /// * `Ok(())` if the timer was removed successfully.
    /// * `Err` if no timer was found for the specified window.
    pub fn remove_timer(&self, border: &Border) -> AnyResult<()> {
        let hwnd_u = border.border_window as usize;
        let shard_index = self.get_shard_index(hwnd_u);

        // Attempt to acquire the lock safely
        let mut timers = self.timers[shard_index]
            .lock()
            .map_err(|e| anyhow!("failed to acquire lock for timers: {e}"))?;

        if let Some(timer) = timers.remove(&hwnd_u) {
            if timer.0.load(Ordering::SeqCst) {
                timer.0.store(false, Ordering::SeqCst);
            }
            Ok(())
        } else {
            Err(anyhow!("no timer found for hwnd: {}", hwnd_u))
        }
    }
}

/// A timer that sends messages at a specified interval to animate window borders.
#[derive(Debug, Clone)]
pub struct AnimationTimer(Arc<AtomicBool>);

impl PartialEq for AnimationTimer {
    fn eq(&self, other: &Self) -> bool {
        self.0.load(Ordering::SeqCst) == other.0.load(Ordering::SeqCst)
    }
}

impl AnimationTimer {
    /// Starts a new animation timer for a window.
    ///
    /// # Arguments
    /// * `border` - The `Border` to associate the timer with.
    /// * `interval_ms` - The interval in milliseconds between timer ticks.
    ///
    /// # Returns
    /// * A `Result` containing the `AnimationTimer` on success, or an error otherwise.
    pub fn start(border: &mut Border, interval_ms: u64) -> AnyResult<AnimationTimer> {
        // Validate the interval
        if interval_ms == 0 {
            return Err(anyhow!("interval must be greater than 0"));
        }

        let running = Arc::new(AtomicBool::new(true));
        let running_clone = running.clone();
        let border_clone = border.clone();
        spawn(move || {
            let window_sent = HWND(as_ptr!(border_clone.border_window));
            let mut next_tick = Instant::now() + Duration::from_millis(interval_ms);
            while running_clone.load(Ordering::SeqCst) {
                // Send the timer message and schedule next tick
                if Instant::now() >= next_tick {
                    if let Err(e) =
                        WindowsApi::post_message_w(window_sent, WM_APP_TIMER, WPARAM(0), LPARAM(0))
                    {
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

        let timer = Self(running);

        TIMER_MANAGER
            .lock()
            .map_err(|e| anyhow!("failed to lock the TIMER_MANAGER: {e}"))?
            .add_timer(border, timer.clone())
            .log_if_err();

        border.animations.timer = Some(timer.clone());
        border.last_animation_time = Some(Instant::now());

        Ok(timer)
    }

    /// Stops the timer of a window from sending further messages.
    ///
    /// # Returns
    /// * `Ok(())` if the timer was stopped successfully.
    pub fn stop(border: &mut Border) -> AnyResult<()> {
        TIMER_MANAGER
            .lock()
            .map_err(|e| anyhow!("failed to lock the TIMER_MANAGER: {e}"))?
            .remove_timer(border)
            .log_if_err();

        border.animations.timer = None;
        Ok(())
    }
}

#[allow(non_snake_case)]
/// Sets an animation timer for the provided `Border` if needed.
///
/// This function checks an optional condition, and if the condition is met or not provided,
/// it starts the animation timer if one is not already running.
///
/// # Arguments
/// * `border` - A mutable reference to the `Border` to set the timer for.
/// * `condition` - An optional condition function that must return `true` for the timer to be set.
///
/// # Returns
/// * `Ok(())` if the timer was set successfully, or an error if the timer could not be started.
pub fn SetAnimationTimer<F>(border: &mut Border, condition: Option<F>) -> AnyResult<()>
where
    F: Fn(&Border) -> bool,
{
    // If condition exists, check it; otherwise, proceed directly
    if condition.is_none_or(|cond| cond(border)) && border.animations.timer.is_none() {
        let timer_duration = (1000.0 / border.animations.fps as f32) as u64;
        AnimationTimer::start(border, timer_duration).log_if_err();
    }
    Ok(())
}

#[allow(non_snake_case)]
/// Kills the animation timer for the provided `Border`.
///
/// This function stops and removes the animation timer for the specified window border.
///
/// # Arguments
/// * `border` - A mutable reference to the `Border` to remove the timer from.
///
/// # Returns
/// * `Ok(())` if the timer was successfully killed, or an error if stopping the timer failed.
pub fn KillAnimationTimer(border: &mut Border) -> AnyResult<()> {
    if border.animations.timer.is_some() {
        AnimationTimer::stop(border).log_if_err();
    }
    Ok(())
}
