use anyhow::anyhow;
use notify_win_debouncer_full::new_debouncer;
use notify_win_debouncer_full::notify_win::Error as NotifyError;
use notify_win_debouncer_full::notify_win::RecursiveMode;
use notify_win_debouncer_full::DebouncedEvent;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;

use crate::user_config::UserConfig;

#[derive(Debug, Clone)]
pub struct ConfigWatcher {
    config_path: PathBuf,
    running: Arc<AtomicBool>,
    timeout: Duration,
    debounce: Duration,
}

impl ConfigWatcher {
    pub fn new(config_path: PathBuf, timeout: Duration) -> Self {
        let running = AtomicBool::new(false);

        Self {
            config_path,
            running: Arc::new(running),
            timeout,
            debounce: Duration::from_millis(500),
        }
    }

    fn handle_events(result: Result<Vec<DebouncedEvent>, Vec<NotifyError>>) {
        if let Ok(events) = result {
            for event in events {
                if event.kind.is_modify() {
                    let is_reloaded = UserConfig::reload();
                    if is_reloaded {
                        break;
                    }
                }
            }
        } else {
            error!("failed to handle events: {:?}", result.err());
        }
    }

    pub fn start(&mut self) -> anyhow::Result<()> {
        if !self.config_path.exists() {
            return Err(anyhow!(
                "configuration file does not exist: {}",
                self.config_path.display()
            ));
        }

        if self.running.swap(true, Ordering::SeqCst) {
            return Err(anyhow!("config watcher is already running"));
        }

        debug!("configuration watcher has started.");

        let running = Arc::clone(&self.running);
        let config_path = self.config_path.clone();
        let timeout = self.timeout;
        let debounce = self.debounce;

        let _ = std::thread::spawn({
            move || -> anyhow::Result<()> {
                let mut debouncer = new_debouncer(timeout, None, Self::handle_events)?;

                debug!(
                    "watching configuration file: {}",
                    config_path.display().to_string()
                );
                debouncer.watch(config_path.as_path(), RecursiveMode::Recursive)?;

                let mut now = Instant::now();
                loop {
                    if !running.load(Ordering::SeqCst) {
                        break;
                    }

                    if now.elapsed() < debounce {
                        std::thread::sleep(debounce - now.elapsed());
                    }
                    now = Instant::now();
                }

                debug!("configuration watcher detected stop flag. Preparing to exit.");
                debouncer.unwatch(config_path.as_path())?;
                Ok(())
            }
        });

        Ok(())
    }

    pub fn stop(&mut self) -> anyhow::Result<()> {
        if !self.running.load(Ordering::SeqCst) {
            debug!("config watcher is not running; skipping cleanup");
        } else {
            debug!("stopping configuration watcher...");
            self.running.store(false, Ordering::SeqCst);
        }
        Ok(())
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }
}

impl Drop for ConfigWatcher {
    fn drop(&mut self) {
        let _ = self.stop(); // Ensure cleanup on drop
    }
}
