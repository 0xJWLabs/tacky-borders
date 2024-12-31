use anyhow::anyhow;
use anyhow::Result as AnyResult;
use std::thread;

#[derive(Debug)]
pub struct ThreadHandle<T>(Option<thread::JoinHandle<T>>);

impl<T> ThreadHandle<T> {
    pub fn new(handle: Option<thread::JoinHandle<T>>) -> Self {
        Self(handle)
    }

    pub fn cast(&mut self, handle: thread::JoinHandle<T>) {
        self.0 = Some(handle);
    }

    pub fn join(&mut self) -> AnyResult<T> {
        if let Some(handle) = self.0.take() {
            handle
                .join()
                .map_err(|e| anyhow!("failed to join thread: {e:?}"))
        } else {
            Err(anyhow!("thread already joined or missing"))
        }
    }
}
