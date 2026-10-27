//! Stitch generation: running, satin, tatami fill, underlay, pull
//! compensation, routing, lock stitches and stitch-length limits.
//!
//! Each stitch type is a pure, deterministic function: geometry and
//! parameters in, a stitch list out. Use `emb_geom::math` for all
//! transcendental functions (see "Determinism across platforms" in
//! `docs/SPEC.md`).
//!
//! Phase 1b fills this crate. When you port an algorithm from Ink/Stitch,
//! put the source file path and the commit hash at the top of the file.
#![forbid(unsafe_code)]

pub mod applique;
pub mod column;
pub mod cross;
pub mod curves;
pub mod effects;
pub mod fill;
pub mod lace;
pub mod meander;
pub mod motifs;
pub mod network;
pub mod object;
pub mod order;
pub mod plan;
pub mod rings;
pub mod ripple;
pub mod running;
pub mod satin;
pub mod tartan;
pub mod textures;
pub mod travel;

pub use object::{
    MAX_OBJECT_MM, ObjectStitches, Run, RunLink, StitchCache, StitchError, StopReason, is_puff, stitch_object,
    stitch_subpaths, too_large,
};
pub use plan::{ObjectSpan, PlanOptions, PlanStop, PlanWarning, StitchPlan, build_plan};
