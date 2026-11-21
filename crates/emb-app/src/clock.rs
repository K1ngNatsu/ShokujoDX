//! The clock. On the desktop these are the types of `std::time`. In a
//! browser (the browser trial of P15) `std::time::Instant::now()` stops the
//! program; `web_time` reads the clock of the browser.

#[cfg(not(target_arch = "wasm32"))]
pub use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[cfg(target_arch = "wasm32")]
pub use web_time::{Duration, Instant, SystemTime, UNIX_EPOCH};
