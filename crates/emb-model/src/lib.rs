//! Design data model for the embroidery digitizer.
//!
//! - [`Design`]: the objects, threads, hoop and rotation of one design.
//! - [`Document`]: a design plus its undo and redo history. All edits go through it.
//! - [`StitchDesign`]: a flat stitch list, the data that machine formats read and write.
//! - [`ThreadChart`]: thread color charts; [`Colorway`]: color versions of a design.
//! - [`HoopCatalog`], [`MachineCatalog`] and [`FabricCatalog`]: presets from
//!   `data/hoops.toml`, `data/machines.toml` and `data/fabrics.toml`.
//! - [`Style`] and [`CurrentProperties`]: stitch settings to apply to objects;
//!   [`template`]: designs that new designs start from.
//! - [`Lettering`]: text as letters made of satin and fill parts.
//! - [`project`]: the native project file format.
#![forbid(unsafe_code)]

pub mod applique;
pub mod arrange;
pub mod backdrop;
pub mod baseline;
pub mod bling;
pub mod chart;
pub mod colorway;
pub mod cross;
pub mod cutwork;
pub mod decor;
pub mod design;
pub mod edit;
pub mod editing;
pub mod envelope;
pub mod fabric;
pub mod history;
pub mod hoop;
pub mod hooping;
pub mod import;
pub mod keyboard;
pub mod lace;
pub mod lettering;
pub mod link;
pub mod machine;
pub mod monogram;
pub mod motif;
pub mod needles;
pub mod object;
pub mod order;
pub mod product;
pub mod project;
pub mod quilt;
pub mod quote;
pub mod repeat;
pub mod sequence;
pub mod shaping;
pub mod sketch;
pub mod sketch_link;
pub mod split;
pub mod stitch;
pub mod stock;
pub mod style;
pub mod tartan;
pub mod teamnames;
pub mod template;
pub mod thread;
pub mod thread_use;
pub mod trace;
pub mod vector;

pub use applique::{AppliqueParams, AppliquePiece, DEFAULT_FABRIC_COLOR, TackDown, outline_edges};
pub use arrange::{Arrangement, BorderPlace, OffsetMake, OffsetSpec};
pub use backdrop::Backdrop;
pub use baseline::Baseline;
pub use bling::{Bling, BlingFill, BlingLayout, Stone};
pub use chart::{ChartThread, Lab, ThreadChart};
pub use colorway::{Colorway, ThreadLook};
pub use cross::{CrossOrder, CrossParams, CrossStitch};
pub use decor::{BlendRecord, ButtonholeSpec, Frame, FrameShape, FrameStitch};
pub use design::{Anchor, Design, Guide, HoopPlacement, HoopSelection, Rotation, StartEnd};
pub use envelope::{Envelope, EnvelopeShape};
pub use fabric::{Fabric, FabricCatalog, FabricChange, UnderlayWeight};
pub use history::{Document, MergeKey};
pub use hoop::{HoopCatalog, HoopPosition, HoopPreset, HoopShape};
pub use hooping::{Hooping, HoopingLayout, HoopingOut};
pub use import::design_from_stitches;
pub use keyboard::{KeyDesign, KeyObject, KeyboardCollection};
pub use lettering::{
    FontKind, FontRef, Justify, Letter, LetterConnection, LetterEdit, LetterOrder, LetterShape, LetterStitch, Lettering,
};
pub use link::{LinkOp, ShapeLink};
pub use machine::{MachineCatalog, MachineProfile, SewTimes};
pub use monogram::{
    Border, BorderShape, BorderStitch, MonogramRecord, MonogramSpec, MonogramStyle, MonogramTemplate, OrnamentPlace,
    OrnamentShape, Ornaments,
};
pub use motif::{Motif, MotifRun};
pub use needles::{NeedlePin, NeedlePlan, NeedleSetup};
pub use object::{
    Accordion, ColumnPattern, Corner, CrossStyle, DesignObject, DirectionLine, FillParams, FillPattern, Functions,
    LineBorder, MeanderTile, ObjectId, ObjectKind, Ripple, RunPass, RunStyle, RunningParams, SatinGeometry,
    SatinParams, TrimMode, Underlay, UnderlayLayer,
};
pub use product::{Placement, ProductKind, ProductPart, ProductPicture, ProductView, Shade};
pub use repeat::RepeatSpec;
pub use sequence::Step;
pub use shaping::{ShapeError, ShapeOp, ShapeOptions};
pub use sketch_link::{LinkSource, SketchLink};
pub use split::{SplitPattern, Stamp, StampPlacement};
pub use stitch::{StitchDesign, StitchKind, StitchRecord};
pub use style::{CurrentProperties, Style};
pub use tartan::Sett;
pub use teamnames::{TeamColumn, TeamNames, TeamRow};
pub use thread::{Rgb, Thread, ThreadId};
pub use trace::{BorderTrace, TraceOrder, TraceSpec, TraceStitch};
pub use vector::{ConvertTo, VectorStyle};

/// Errors from loading data files and project files.
#[derive(Debug, thiserror::Error)]
pub enum ModelError {
    #[error("cannot parse {what}: {source}")]
    Toml {
        what: &'static str,
        #[source]
        source: toml::de::Error,
    },
    #[error("cannot parse project file: {0}")]
    Json(#[from] serde_json::Error),
    #[error("invalid {what}: {message}")]
    Invalid { what: &'static str, message: String },
    #[error("unsupported project file: {0}")]
    UnsupportedProject(String),
    #[error("cannot read {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
}
