use anyhow::anyhow;
use notify_win_debouncer_full::DebouncedEvent;
use notify_win_debouncer_full::new_debouncer;
use notify_win_debouncer_full::notify_win;
use notify_win_debouncer_full::notify_win::RecursiveMode;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::thread;
use std::time::Duration;
use std::time::Instant;

use crate::user_config::UserConfig;

#[derive(Debug)]
pub struct ThreadHandle<T>(Option<thread::JoinHandle<anyhow::Result<T>>>);

impl<T> ThreadHandle<T> {
    pub fn new(handle: Option<thread::JoinHandle<anyhow::Result<T>>>) -> Self {
        Self(handle)
    }

    pub fn cast(&mut self, handle: thread::JoinHandle<anyhow::Result<T>>) {
        self.0 = Some(handle);
    }

    pub fn join(&mut self) -> anyhow::Result<T> {
        if let Some(handle) = self.0.take() {
            handle
                .join()
                .map_err(|e| anyhow!("[join] Thread Handle: Thread panicked: {:?}", e))?
        } else {
            Err(anyhow!(
                "[join] Thread Handle: Already joined or no thread available"
            ))
        }
    }
}

impl<T> Drop for ThreadHandle<T> {
    /// Ensures that the thread is properly joined or dropped when the `ThreadHandle` is dropped.
    fn drop(&mut self) {
        if self.0.is_some() {
            error!("[drop] Thread Handle: Dropped without being joined!");
        }
    }
}

#[derive(Debug)]
pub struct ConfigWatcher {
    config_path: PathBuf,
    running: Arc<AtomicBool>,
    timeout: Duration,
    thread: ThreadHandle<()>,
}

impl ConfigWatcher {
    pub fn new(config_path: PathBuf, timeout: Duration) -> Self {
        Self {
            config_path,
            running: Arc::new(AtomicBool::new(false)),
            timeout,
            thread: ThreadHandle::new(None),
        }
    }

    fn handle_events(result: Result<Vec<DebouncedEvent>, Vec<notify_win::Error>>) {
        match result {
            Ok(events) => {
                for event in events {
                    if event.kind.is_modify() {
                        let is_reloaded = UserConfig::reload();
                        if is_reloaded {
                            break;
                        }
                    }
                }
            }
            Err(err) => {
                error!("[handle_events] Config Watcher: failed to handle events (error: {err:?})")
            }
        }
    }

    pub fn start(&mut self) -> anyhow::Result<()> {
        if !self.config_path.exists() {
            return Err(anyhow!(
                "[start] Config Watcher: Configuration file does not exist: {}",
                self.config_path.display()
            ));
        }

        if self.running.swap(true, Ordering::SeqCst) {
            return Err(anyhow!("[start] Config Watcher: Is already running"));
        }

        let running = Arc::clone(&self.running);
        let config_path = self.config_path.clone();
        let timeout = self.timeout;
        let debounce = Duration::from_millis(500);

        let handle = thread::spawn({
            move || -> anyhow::Result<()> {
                let mut debouncer =
                    new_debouncer(timeout, None, Self::handle_events).map_err(|e| {
                        anyhow!(
                            "[start] Config Watcher: Failed to create debouncer (error: {:?})",
                            e
                        )
                    })?;

                debug!(
                    "[start] Config Watcher: Watching (File: {})",
                    config_path.display().to_string()
                );

                debouncer
                    .watch(config_path.as_path(), RecursiveMode::Recursive)
                    .map_err(|e| {
                        anyhow!(
                            "[start] Config Watcher: Failed to watch config path: (error: {:?})",
                            e
                        )
                    })?;

                let mut last_checked = Instant::now();

                while running.load(Ordering::SeqCst) {
                    let elapsed = last_checked.elapsed();
                    if elapsed < debounce {
                        thread::sleep(debounce - elapsed);
                    }
                    last_checked = Instant::now();
                }

                debouncer.unwatch(config_path.as_path()).map_err(|e| {
                    anyhow!("[start] Config Watcher: Failed to unwatch (error: {:?})", e)
                })?;

                debug!("[start] Config Watcher: Stopped");
                Ok(())
            }
        });

        self.thread.cast(handle);

        debug!("[start] Config Watcher: Started");

        Ok(())
    }

    pub fn stop(&mut self) -> anyhow::Result<()> {
        if !self.running.load(Ordering::SeqCst) {
            debug!("[stop] Config Watcher: Is not running; skipping cleanup");
            return Ok(());
        }
        debug!("[stop] Config Watcher: Stopping");
        self.running.store(false, Ordering::SeqCst);
        self.thread.join()?;

        Ok(())
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }
}

impl Drop for ConfigWatcher {
    fn drop(&mut self) {
        if let Err(err) = self.stop() {
            debug!("[drop] Config Watcher: Error stopping ConfigWatcher (error: {err:?})");
        }
    }
}
