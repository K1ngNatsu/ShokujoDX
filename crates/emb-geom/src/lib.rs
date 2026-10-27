//! 2D geometry for the embroidery engine.
//!
//! All lengths are in millimeters as `f64`. The Y axis points down.
//! This crate has no dependency on other project crates.
#![forbid(unsafe_code)]

pub mod bbox;
pub mod math;
pub mod nodes;
pub mod path;
pub mod point;
pub mod region;
pub mod simplify;
pub mod spine;
pub mod transform;

pub use bbox::BBox;
pub use path::{Path, PathCmd, Polyline};
pub use point::Point;
pub use region::{Join, Region};
pub use transform::Transform;

/// Default tolerance in mm to flatten curves to polylines.
pub const DEFAULT_FLATTEN_TOLERANCE_MM: f64 = 0.05;
