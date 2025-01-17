#![allow(dead_code)]

use crate::error::LogIfErr;
use crate::windows_api::PointerConversion;
use crate::windows_api::WM_APP_TIMER;
use crate::windows_api::WindowsApi;
use anyhow::anyhow;
#[cfg(feature = "fast-hash")]
use fx_hash::{FxHashMap as HashMap, FxHashMapExt};
#[cfg(not(feature = "fast-hash"))]
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock, Mutex};
use std::thread::{sleep, spawn};
use std::time::{Duration, Instant};
use windows::Win32::Foundation::{LPARAM, WPARAM};

const NUM_SHARDS: isize = 16;

/// Global custom timer manager.
/// Manages and coordinates timers for windows.
pub static TIMER_MANAGER: LazyLock<Arc<Mutex<CustomTimerManager>>> =
    LazyLock::new(|| Arc::new(Mutex::new(CustomTimerManager::new())));

/// A manager for custom timers, ensuring timers are associated with specific windows.
#[derive(Debug)]
pub struct CustomTimerManager {
    /// Map of timers keyed by window handle (as `usize`).
    timers: Vec<Arc<Mutex<HashMap<isize, CustomTimer>>>>,
}

impl CustomTimerManager {
    /// Creates a new instance of `CustomTimerManager` with empty timers.
    ///
    /// # Returns
    /// * A new instance of `CustomTimerManager` with initialized timer shards.
    pub fn new() -> Self {
        let mut timers = Vec::with_capacity(NUM_SHARDS as usize);
        for _ in 0..NUM_SHARDS {
            timers.push(Arc::new(Mutex::new(HashMap::new())));
        }

        Self { timers }
    }

    /// Determines the shard index for a given window handle.
    ///
    /// # Arguments
    /// * `hwnd` - The window handle.
    ///
    /// # Returns
    /// * The index of the shard.
    fn get_shard_index(&self, hwnd: isize) -> usize {
        (hwnd % NUM_SHARDS) as usize
    }

    /// Adds a new timer for a specific window.
    ///
    /// # Arguments
    /// * `hwnd` - The window handle to associate with the timer.
    /// * `timer` - The `CustomTimer` to be added.
    ///
    /// # Returns
    /// * `Ok(())` if the timer was successfully added.
    /// * `Err` if a timer already exists for the specified window.
    pub fn add_timer(&self, hwnd: isize, timer: CustomTimer) -> anyhow::Result<()> {
        let shard_index = self.get_shard_index(hwnd);
        // Attempt to acquire the lock safely
        let mut timers = self.timers[shard_index]
            .lock()
            .map_err(|e| anyhow!("failed to acquire lock for timers: {e}"))?;

        if let Entry::Vacant(e) = timers.entry(hwnd) {
            e.insert(timer.clone());
            Ok(())
        } else {
            Err(anyhow!("timer with this hwnd already exists."))
        }
    }

    /// Removes the timer associated with a specific window.
    ///
    /// # Arguments
    /// * `hwnd` - The window handle to disassociate from the timer.
    ///
    /// # Returns
    /// * `Ok(())` if the timer was removed successfully.
    /// * `Err` if no timer was found for the specified window.
    pub fn remove_timer(&self, hwnd: isize) -> anyhow::Result<()> {
        let shard_index = self.get_shard_index(hwnd);

        // Attempt to acquire the lock safely
        let mut timers = self.timers[shard_index]
            .lock()
            .map_err(|e| anyhow!("failed to acquire lock for timers: {e}"))?;

        if let Some(timer) = timers.remove(&hwnd) {
            if timer.0.load(Ordering::SeqCst) {
                timer.0.store(false, Ordering::SeqCst);
            }
            Ok(())
        } else {
            Err(anyhow!("no timer found for hwnd: {}", hwnd))
        }
    }
}

/// A timer that sends messages at a specified interval to animate window borders.
#[derive(Debug, Clone)]
pub struct CustomTimer(Arc<AtomicBool>);

impl PartialEq for CustomTimer {
    fn eq(&self, other: &Self) -> bool {
        self.0.load(Ordering::SeqCst) == other.0.load(Ordering::SeqCst)
    }
}

impl CustomTimer {
    /// Starts a new custom timer for a window.
    ///
    /// # Arguments
    /// * `hwnd` - The window handle to associate with the timer.
    /// * `interval_ms` - The interval in milliseconds between timer ticks.
    ///
    /// # Returns
    /// * `Ok(CustomTimer)` if the timer was successfully started.
    /// * `Err` if the interval is invalid (i.e., 0) or there was an error during timer setup.
    pub fn start(hwnd: isize, interval_ms: u64) -> anyhow::Result<CustomTimer> {
        // Validate the interval
        if interval_ms == 0 {
            return Err(anyhow!("interval must be greater than 0"));
        }

        let running = Arc::new(AtomicBool::new(true));
        let running_clone = running.clone();
        spawn(move || {
            let window_sent = hwnd.as_hwnd();
            let mut next_tick = Instant::now() + Duration::from_millis(interval_ms);
            while running_clone.load(Ordering::SeqCst) {
                // Send the timer message and schedule next tick
                if Instant::now() >= next_tick {
                    if let Err(e) = WindowsApi::post_message_w(
                        Some(window_sent),
                        WM_APP_TIMER,
                        WPARAM(0),
                        LPARAM(0),
                    ) {
                        error!("could not send timer message: {e}");
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
            .add_timer(hwnd, timer.clone())
            .log_if_err();

        Ok(timer)
    }

    /// Stops the timer of a window from sending further messages.
    ///
    /// # Arguments
    /// * `hwnd` - The window handle whose timer should be stopped.
    ///
    /// # Returns
    /// * `Ok(())` if the timer was successfully stopped.
    /// * `Err` if an error occurred during the stopping process.
    pub fn stop(hwnd: isize) -> anyhow::Result<()> {
        TIMER_MANAGER
            .lock()
            .map_err(|e| anyhow!("failed to lock the TIMER_MANAGER: {e}"))?
            .remove_timer(hwnd)
            .log_if_err();

        Ok(())
    }
}
