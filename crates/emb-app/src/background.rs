//! Background jobs. On the desktop, [`spawn`] starts a thread. A browser
//! (the browser trial of P15) cannot start threads: there the job runs at
//! once, and the frame waits for it. The jobs of the program send their
//! results through a channel, so they do not wait for the caller.

/// The handle of a job.
pub struct Handle<T> {
    #[cfg(not(target_arch = "wasm32"))]
    inner: std::thread::JoinHandle<T>,
    #[cfg(target_arch = "wasm32")]
    value: T,
}

impl<T> Handle<T> {
    pub fn is_finished(&self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        return self.inner.is_finished();
        #[cfg(target_arch = "wasm32")]
        return true;
    }

    pub fn join(self) -> std::thread::Result<T> {
        #[cfg(not(target_arch = "wasm32"))]
        return self.inner.join();
        #[cfg(target_arch = "wasm32")]
        return Ok(self.value);
    }
}

/// Starts `f` on a new thread (in the browser: runs it now).
pub fn spawn<F, T>(f: F) -> Handle<T>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    #[cfg(not(target_arch = "wasm32"))]
    return Handle {
        inner: std::thread::spawn(f),
    };
    #[cfg(target_arch = "wasm32")]
    return Handle { value: f() };
}
