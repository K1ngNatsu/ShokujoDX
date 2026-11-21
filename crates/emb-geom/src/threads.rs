//! Scoped threads for the engine crates. On the desktop, [`scope`] is
//! `std::thread::scope`. A browser (`wasm32`, the browser trial of P15)
//! cannot start threads: there each spawned job runs at once, on the thread
//! of the caller, and gives the same result. The jobs of the engine share
//! only their inputs and an atomic counter, so the order does not change
//! the result.

#[cfg(not(target_arch = "wasm32"))]
pub use std::thread::scope;

#[cfg(target_arch = "wasm32")]
pub use one_thread::{Scope, ScopedJoinHandle, scope};

#[cfg(target_arch = "wasm32")]
mod one_thread {
    use std::marker::PhantomData;

    /// The scope of [`scope`]: its jobs run when they are spawned.
    pub struct Scope<'scope, 'env: 'scope> {
        _scope: PhantomData<&'scope mut &'scope ()>,
        _env: PhantomData<&'env mut &'env ()>,
    }

    /// The result of a job that ran.
    pub struct ScopedJoinHandle<'scope, T> {
        value: T,
        _scope: PhantomData<&'scope ()>,
    }

    impl<T> ScopedJoinHandle<'_, T> {
        pub fn join(self) -> std::thread::Result<T> {
            Ok(self.value)
        }

        pub fn is_finished(&self) -> bool {
            true
        }
    }

    impl<'scope> Scope<'scope, '_> {
        pub fn spawn<F, T>(&'scope self, f: F) -> ScopedJoinHandle<'scope, T>
        where
            F: FnOnce() -> T + Send + 'scope,
            T: Send + 'scope,
        {
            ScopedJoinHandle {
                value: f(),
                _scope: PhantomData,
            }
        }
    }

    pub fn scope<'env, F, T>(f: F) -> T
    where
        F: for<'scope> FnOnce(&'scope Scope<'scope, 'env>) -> T,
    {
        let s = Scope {
            _scope: PhantomData,
            _env: PhantomData,
        };
        f(&s)
    }
}
