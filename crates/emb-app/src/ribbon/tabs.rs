//! The panels of each tab (and of simple mode), and the list of all
//! commands with their tab.

use super::*;

// ---- Layout of the tabs ----

/// One part of a panel.
pub(super) enum Item {
    /// A large button: icon above the name.
    Large(Cmd),
    /// Up to three small buttons, one above the other.
    Small(&'static [Cmd]),
    /// Controls that are not plain buttons, and the commands that they
    /// show (for the help and the tests).
    Custom(fn(&mut egui::Ui, &mut AppState, &SelInfo), &'static [Cmd]),
}

pub(super) struct Panel {
    pub(super) title: &'static str,
    pub(super) items: &'static [Item],
}

pub(super) use Item::{Custom, Large, Small};

pub(super) const fn t(tool: Tool) -> Cmd {
    Cmd::Tool(tool)
}

pub(super) const fn f(flag: Flag) -> Cmd {
    Cmd::Toggle(flag)
}

pub(super) const DIGITIZE: &[Panel] = &[
    Panel {
        title: "Select",
        items: &[
            Large(t(Tool::Select)),
            Large(t(Tool::Reshape)),
            Small(&[t(Tool::Lasso), t(Tool::PolySelect), t(Tool::LineSelect)]),
        ],
    },
    Panel {
        title: "Lines",
        items: &[
            Large(t(Tool::Line)),
            Small(&[t(Tool::Manual), t(Tool::Freehand), t(Tool::SmartDraw)]),
        ],
    },
    Panel {
        title: "Fills",
        items: &[
            Large(t(Tool::Shape)),
            Small(&[t(Tool::ComplexFill), t(Tool::FreeShape), t(Tool::Turning)]),
            Small(&[t(Tool::SmartFill)]),
        ],
    },
    Panel {
        title: "Columns",
        items: &[
            Large(t(Tool::Column)),
            Small(&[t(Tool::ColumnB), t(Tool::ColumnC), t(Tool::Ring)]),
        ],
    },
    Panel {
        title: "Shapes",
        items: &[
            Small(&[t(Tool::Rectangle), t(Tool::RoundRect), t(Tool::Ellipse)]),
            Small(&[t(Tool::Star), Cmd::StartSketch]),
        ],
    },
    Panel {
        title: "Decorate",
        items: &[Large(Cmd::Borders), Small(&[Cmd::Buttonholes, Cmd::Blends])],
    },
    Panel {
        title: "Appliqué",
        items: &[
            Large(t(Tool::Applique)),
            Small(&[Cmd::ConvertToApplique, Cmd::AppliqueFromSelection, Cmd::CombineApplique]),
            Small(&[Cmd::BackToOriginal, Cmd::CutterFile]),
        ],
    },
];

/// Lettering and personalization.
pub(super) const LETTERING: &[Panel] = &[
    Panel {
        title: "Lettering",
        items: &[
            Large(t(Tool::Lettering)),
            Small(&[Cmd::Fonts, Cmd::Alphabets, Cmd::LetteringOnLine]),
        ],
    },
    Panel {
        title: "Personalize",
        items: &[
            Large(Cmd::Monogram),
            Small(&[Cmd::TeamNames, Cmd::KeyboardDesigns]),
            Large(Cmd::Kiosk),
        ],
    },
];

/// Specialty decoration (P9, P11): cross stitch, rhinestones, cutwork,
/// quilting, puff and laydown. Each has its own panel.
pub(super) const SPECIALTY: &[Panel] = &[
    Panel {
        title: "Cross Stitch",
        items: &[
            Large(t(Tool::CrossStitch)),
            Small(&[t(Tool::CrossErase), t(Tool::Backstitch), Cmd::ConvertToCross]),
            Large(Cmd::PictureToCross),
            Large(Cmd::CrossChart),
        ],
    },
    Panel {
        title: "Rhinestones",
        items: &[
            Large(t(Tool::StoneRun)),
            Small(&[t(Tool::Stone), t(Tool::StoneFill)]),
            Small(&[Cmd::ToStonesRun, Cmd::ToStonesCenter, Cmd::ToStonesFill]),
            Small(&[Cmd::ToSingleStones, f(Flag::Stones)]),
            Large(Cmd::StoneTemplate),
            Small(&[Cmd::TemplateMarks]),
        ],
    },
    Panel {
        title: "Cutwork",
        items: &[Large(Cmd::Cutwork)],
    },
    Panel {
        title: "Quilting",
        items: &[Large(Cmd::Quilting)],
    },
    Panel {
        title: "Puff and Laydown",
        items: &[Large(Cmd::Puff), Large(Cmd::Laydown)],
    },
];

/// Pictures and vector artwork (P6): import, picture tools, trace, draw,
/// convert, shaping, borders and offsets, and export. The copies of the
/// selection (arrangements) are on the Arrange tab, quilting is on the
/// Specialty tab (ADR 0055).
pub(super) const fn sk(t: crate::sketch::SketchTool) -> Cmd {
    Cmd::Sketch(SketchCmd::Tool(t))
}

pub(super) const fn kc(k: crate::sketch::Kind) -> Cmd {
    Cmd::Sketch(SketchCmd::Tool(crate::sketch::SketchTool::Constrain(k)))
}

/// The constraint commands of the Sketch tab, as a grid of icons (as the
/// other program shows them; the names are in the tooltips).
pub(super) const SKETCH_CONSTRAINTS: [Cmd; 13] = {
    use crate::sketch::Kind as K;
    [
        kc(K::Coincident),
        kc(K::Collinear),
        kc(K::Concentric),
        kc(K::Fix),
        kc(K::Parallel),
        kc(K::Perpendicular),
        kc(K::Horizontal),
        kc(K::Vertical),
        kc(K::Tangent),
        kc(K::Equal),
        kc(K::Midpoint),
        kc(K::OnCurve),
        kc(K::Symmetric),
    ]
};

/// The grid of the constraint commands: three rows of icons.
pub(super) fn constraint_icons(ui: &mut egui::Ui, st: &mut AppState, s: &SelInfo) {
    for row in SKETCH_CONSTRAINTS.chunks(5) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            for cmd in row {
                icon_cmd(ui, st, s, *cmd);
            }
        });
    }
}

/// A command as an icon only (the name is the tooltip and the screen
/// reader name).
pub(super) fn icon_cmd(ui: &mut egui::Ui, st: &mut AppState, s: &SelInfo, cmd: Cmd) {
    let (icon, label, _) = cmd.meta();
    let enabled = st.can(cmd, s);
    let on = st.is_on(cmd);
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(SMALL_H), sense(enabled));
    let resp = crate::a11y::named(resp, label);
    note_focus(ui, &resp);
    note_rect(label, rect);
    frame(ui, rect, &resp, on, enabled);
    let (fg, bg) = colors(ui, enabled);
    icons::paint(ui.painter(), rect.shrink(3.0), icon, fg, bg);
    let ctx = ui.ctx().clone();
    let resp = tooltip(resp, &ctx, cmd);
    if resp.hovered() {
        st.hover_cmd = Some(cmd);
    }
    if resp.clicked() {
        st.run(cmd, &ctx);
    }
}

/// The Sketch tab (sketch mode, P14 Milestone 2).
pub(super) const SKETCH: &[Panel] = {
    use crate::sketch::SketchTool as T;
    &[
        Panel {
            title: "Sketch",
            items: &[
                Large(Cmd::Sketch(SketchCmd::Finish)),
                Large(sk(T::Select)),
                Large(sk(T::Regions)),
            ],
        },
        Panel {
            title: "Draw",
            items: &[
                Large(sk(T::Line)),
                Large(sk(T::Spline)),
                Small(&[sk(T::Rectangle), sk(T::RectCenter), sk(T::Rect3)]),
                Small(&[sk(T::RectCenter3), sk(T::Slot), sk(T::Polygon)]),
                Small(&[sk(T::Circle), sk(T::TangentCircle), sk(T::Point)]),
                Small(&[sk(T::Arc3), sk(T::ArcCenter), sk(T::ArcTangent)]),
                Small(&[
                    Cmd::Sketch(SketchCmd::Construction),
                    Cmd::Sketch(SketchCmd::Centerline),
                    Cmd::Sketch(SketchCmd::CenterPoint),
                ]),
            ],
        },
        Panel {
            title: "Constrain",
            items: &[Custom(constraint_icons, &SKETCH_CONSTRAINTS)],
        },
        Panel {
            title: "Dimension",
            items: &[
                Large(sk(T::Dimension)),
                Large(Cmd::Sketch(SketchCmd::Parameters)),
                Small(&[
                    Cmd::Sketch(SketchCmd::AutoDimension),
                    Cmd::Sketch(SketchCmd::Relax),
                    Cmd::Sketch(SketchCmd::RelaxMode),
                ]),
            ],
        },
        Panel {
            title: "Modify",
            items: &[
                Small(&[sk(T::Trim), sk(T::Extend), sk(T::Split)]),
                Small(&[sk(T::Fillet), sk(T::Chamfer), sk(T::Offset)]),
                Small(&[sk(T::Move), sk(T::Copy), sk(T::Rotate)]),
                Small(&[sk(T::Scale), sk(T::Stretch)]),
            ],
        },
        Panel {
            title: "Copy",
            items: &[Small(&[sk(T::Mirror), sk(T::RectPattern), sk(T::CircularPattern)])],
        },
    ]
};

pub(super) const ARTWORK: &[Panel] = &[
    Panel {
        title: "Import",
        items: &[
            Large(Cmd::PictureWizard),
            Large(Cmd::ImportSvgVectors),
            Small(&[Cmd::LoadPicture, Cmd::ImportSvg, Cmd::ImportSvgLines]),
            Small(&[Cmd::InsertMachineFile]),
        ],
    },
    Panel {
        title: "Picture",
        items: &[
            Large(Cmd::PictureTools),
            Small(&[t(Tool::CropRect), t(Tool::CropEllipse), t(Tool::CropPolygon)]),
            Small(&[Cmd::EditPictureOutside]),
        ],
    },
    Panel {
        title: "Trace",
        items: &[
            Large(Cmd::TracePicture),
            Large(t(Tool::ClickStitch)),
            Small(&[
                Cmd::ClickAs(emb_art::click::ClickStitch::Fill),
                Cmd::ClickAs(emb_art::click::ClickStitch::FillNoHoles),
                Cmd::ClickAs(emb_art::click::ClickStitch::Satin),
            ]),
            Small(&[
                Cmd::ClickAs(emb_art::click::ClickStitch::CenterLine),
                Cmd::ClickAs(emb_art::click::ClickStitch::Outline),
            ]),
        ],
    },
    Panel {
        title: "Draw",
        items: &[
            Large(f(Flag::DrawVectors)),
            Small(&[t(Tool::Shape), t(Tool::Line), t(Tool::Freehand)]),
            Small(&[t(Tool::Rectangle), t(Tool::Ellipse), t(Tool::Star)]),
            Small(&[f(Flag::Vectors), f(Flag::DimVectors)]),
        ],
    },
    Panel {
        title: "Convert",
        items: &[
            Large(Cmd::Convert(emb_model::ConvertTo::Fill)),
            Small(&[
                Cmd::Convert(emb_model::ConvertTo::Satin),
                Cmd::Convert(emb_model::ConvertTo::Outline),
                Cmd::ToVector,
            ]),
            Small(&[Cmd::RecognizeOutlines, Cmd::VectorToSketch]),
        ],
    },
    Panel {
        title: "Shaping",
        items: &[
            Large(Cmd::Shape(emb_model::ShapeOp::Weld)),
            Small(&[
                Cmd::Shape(emb_model::ShapeOp::Intersect),
                Cmd::Shape(emb_model::ShapeOp::Exclude),
                Cmd::Shape(emb_model::ShapeOp::Combine),
            ]),
            Small(&[
                Cmd::Shape(emb_model::ShapeOp::FrontMinusBack),
                Cmd::Shape(emb_model::ShapeOp::BackMinusFront),
            ]),
            Small(&[
                Cmd::Shape(emb_model::ShapeOp::Flatten),
                Cmd::Shape(emb_model::ShapeOp::Divide),
                Cmd::RemoveOverlaps,
            ]),
            Small(&[Cmd::Unlink]),
            Custom(shaping_options, &[]),
        ],
    },
    Panel {
        title: "Borders and Offsets",
        items: &[Large(Cmd::BorderTrace), Large(Cmd::Offsets), Small(&[Cmd::AddBorder])],
    },
    Panel {
        title: "Output",
        items: &[Large(Cmd::ExportOutlines)],
    },
];

pub(super) const EDIT: &[Panel] = &[
    Panel {
        title: "Clipboard",
        items: &[
            Large(Cmd::Paste),
            Small(&[Cmd::Cut, Cmd::Copy, Cmd::Duplicate]),
            Small(&[Cmd::PasteAfter, Cmd::PasteSpecial, Cmd::Delete]),
        ],
    },
    Panel {
        title: "Clipart",
        items: &[Large(Cmd::ClipartPanel), Small(&[Cmd::SaveClipart])],
    },
    Panel {
        title: "Select",
        items: &[
            Large(Cmd::SelectAll),
            Small(&[Cmd::SelectNext, Cmd::SelectPrev, Cmd::SelectAtCursor]),
            Custom(select_by, &[]),
        ],
    },
    Panel {
        title: "Split and Join",
        items: &[
            Large(t(Tool::Knife)),
            Small(&[Cmd::BreakApart, Cmd::Join, Cmd::SplitAtNeedle]),
            Small(&[Cmd::CloseGaps, t(Tool::TrimSegment)]),
        ],
    },
    Panel {
        title: "Shape",
        items: &[
            Large(t(Tool::StitchAngle)),
            Small(&[t(Tool::AddHole), Cmd::RemoveHoles, Cmd::FillHoles]),
            Small(&[t(Tool::DirectionLine), Cmd::CornerRadius, Cmd::HoleFromOutline]),
            Small(&[Cmd::QuickSimplify, Cmd::Simplify, Cmd::CleanUp]),
            Small(&[Cmd::Envelope, Cmd::CopyProperties, Cmd::RepeatLast]),
        ],
    },
    Panel {
        title: "Lines and Columns",
        items: &[
            Small(&[Cmd::ContinueLine, Cmd::Redwork, Cmd::Branching]),
            Small(&[Cmd::CloseShape(false), Cmd::CloseShape(true), Cmd::Reverse]),
            Small(&[Cmd::SwapSides]),
        ],
    },
    Panel {
        title: "Stitch Types",
        items: &[
            Custom(stitch_types, &[]),
            Small(&[Cmd::Motifs, Cmd::FlorentineGuide, Cmd::SplitLines]),
            Small(&[Cmd::Tartan, Cmd::Gradient, Cmd::GuideLines]),
            Small(&[Cmd::RippleGuide]),
        ],
    },
    Panel {
        title: "Machine Functions",
        items: &[
            Small(&[
                Cmd::Func(Func::Stop),
                Cmd::Func(Func::TrimAlways),
                Cmd::Func(Func::TrimNever),
            ]),
            Small(&[Cmd::Func(Func::TieIn), Cmd::Func(Func::TieOff)]),
        ],
    },
    Panel {
        title: "Scripts",
        items: &[Large(Cmd::Scripts)],
    },
];

pub(super) const ARRANGE: &[Panel] = &[
    Panel {
        title: "Transform",
        items: &[
            Small(&[Cmd::RotateCcw, Cmd::RotateCw, Cmd::Rotate180]),
            Small(&[Cmd::MirrorH, Cmd::MirrorV, Cmd::CenterInHoop]),
            Small(&[t(Tool::MirrorLine), t(Tool::RefPoints)]),
            Custom(rotate_skew, &[]),
        ],
    },
    Panel {
        title: "Align",
        items: &[
            Small(&[
                Cmd::Align(Align::Left),
                Cmd::Align(Align::CenterX),
                Cmd::Align(Align::Right),
            ]),
            Small(&[
                Cmd::Align(Align::Top),
                Cmd::Align(Align::CenterY),
                Cmd::Align(Align::Bottom),
            ]),
        ],
    },
    Panel {
        title: "Space and Size",
        items: &[
            Small(&[Cmd::Space(true), Cmd::Space(false)]),
            Small(&[
                Cmd::SameSize(true, false),
                Cmd::SameSize(false, true),
                Cmd::SameSize(true, true),
            ]),
        ],
    },
    Panel {
        title: "Copies",
        items: &[
            Large(Cmd::Arrangements),
            Small(&[
                Cmd::MirrorMerge(true, false),
                Cmd::MirrorMerge(false, true),
                Cmd::MirrorMerge(true, true),
            ]),
            Small(&[Cmd::Reflect]),
        ],
    },
    Panel {
        title: "Group and Lock",
        items: &[Small(&[Cmd::Group, Cmd::Ungroup]), Small(&[Cmd::Lock, Cmd::UnlockAll])],
    },
    Panel {
        title: "Sew Order",
        items: &[
            Small(&[Cmd::ToStart, Cmd::BackOne, Cmd::ForwardOne]),
            Small(&[Cmd::ToEnd, Cmd::BackColor, Cmd::ForwardColor]),
            Small(&[Cmd::ClickOrder, Cmd::OptimizeColors, Cmd::GroupByColor]),
            Small(&[Cmd::OptimizeOrder, Cmd::CapOrder, Cmd::ClosestJoin]),
        ],
    },
];

pub(super) const COLOR: &[Panel] = &[
    Panel {
        title: "Threads",
        items: &[
            Large(Cmd::ThreadsPanel),
            Small(&[Cmd::RemoveUnused, Cmd::CycleColors, Cmd::SplitThread]),
        ],
    },
    Panel {
        title: "Thread Chart",
        items: &[Custom(chart_pick, &[]), Small(&[Cmd::MatchChart, Cmd::ImportChart])],
    },
    Panel {
        title: "Colorways",
        items: &[
            Custom(colorway_pick, &[]),
            Small(&[Cmd::NewColorway, Cmd::DeleteColorway]),
        ],
    },
    Panel {
        title: "Fabric Color",
        items: &[Custom(background_pick, &[])],
    },
];

pub(super) const VIEW: &[Panel] = &[
    Panel {
        title: "Show",
        items: &[
            Custom(look_column, &[f(Flag::Artwork), f(Flag::Stitches), f(Flag::ThreadLook)]),
            Small(&[f(Flag::Jumps), f(Flag::NeedlePoints), f(Flag::Functions)]),
            Small(&[f(Flag::AppliqueFabric), f(Flag::Vectors), f(Flag::DimVectors)]),
            Small(&[f(Flag::Density)]),
        ],
    },
    Panel {
        title: "Stitch Cursor",
        items: &[
            Large(f(Flag::Player)),
            Large(f(Flag::StitchCursor)),
            Large(Cmd::StitchListPanel),
            Custom(cursor_tools, &[Cmd::CenterAtCursor]),
        ],
    },
    Panel {
        title: "Filter",
        items: &[
            Small(&[Cmd::ShowSelectedColors, Cmd::HideOthers, Cmd::HideSelected]),
            Custom(show_colors, &[Cmd::ShowAll]),
        ],
    },
    Panel {
        title: "Zoom",
        items: &[
            Large(Cmd::ZoomExtents),
            Small(&[Cmd::ZoomIn, Cmd::ZoomOut, Cmd::Zoom100]),
            Small(&[Cmd::ZoomHoop, Cmd::ZoomSelection, t(Tool::Zoom)]),
            Custom(
                views_column,
                &[
                    f(Flag::Overview),
                    f(Flag::SplitView),
                    Cmd::NewView,
                    Cmd::OtherDesign,
                    Cmd::PreviousView,
                ],
            ),
            Custom(zoom_factor, &[Cmd::ZoomTwice(false), Cmd::ZoomTwice(true)]),
        ],
    },
    Panel {
        title: "Workspace",
        items: &[
            Small(&[f(Flag::Grid), f(Flag::Rulers), t(Tool::Measure)]),
            Small(&[t(Tool::ConstructionLine), t(Tool::ConstructionPoint)]),
            Small(&[f(Flag::SnapGrid), f(Flag::SnapGuides), f(Flag::SnapObjects)]),
            Custom(workspace_more, &[Cmd::RemoveGuides, f(Flag::DarkMode)]),
            Large(Cmd::Options),
        ],
    },
];

pub(super) const DESIGN: &[Panel] = &[
    Panel {
        title: "Machine and Hoop",
        items: &[Custom(machine_hoop, &[]), Small(&[Cmd::SewTimes, Cmd::Needles])],
    },
    Panel {
        title: "Hoop",
        items: &[
            Large(Cmd::AutoHoop),
            Large(Cmd::RotateDesign),
            Small(&[Cmd::TurnHoopLeft, Cmd::TurnHoopRight, f(Flag::Hoop)]),
            Small(&[Cmd::MyHoops, Cmd::HoopOptions, Cmd::HoopTemplate]),
        ],
    },
    Panel {
        title: "Fabric and Styles",
        items: &[
            Large(Cmd::Fabric),
            Small(&[Cmd::MakeCurrent, Cmd::ApplyCurrent, Cmd::StylesPanel]),
            Custom(style_picker, &[]),
            Custom(favorites, &[]),
        ],
    },
    Panel {
        title: "Library and Templates",
        items: &[
            Large(Cmd::Library),
            Small(&[Cmd::NewFromTemplate, Cmd::SaveAsTemplate, Cmd::Assets]),
        ],
    },
    Panel {
        title: "Repeats",
        items: &[Large(Cmd::RepeatsWindow), Small(&[f(Flag::Repeats), Cmd::MakeRepeats])],
    },
    Panel {
        title: "Product",
        items: &[
            Large(Cmd::ProductWindow),
            Small(&[f(Flag::Product), Cmd::SaveDecoration]),
        ],
    },
    Panel {
        title: "Orders",
        items: &[Large(Cmd::OrderDetails), Large(Cmd::Quote), Large(Cmd::ThreadStock)],
    },
    Panel {
        title: "Output",
        items: &[
            Large(Cmd::OutputDesign),
            Large(Cmd::Export),
            Large(Cmd::SendToUsb),
            Small(&[Cmd::PrintPreview, Cmd::BatchOutput, Cmd::ExportSvg]),
            Small(&[Cmd::SendToUsbChoose, Cmd::SendByEmail]),
        ],
    },
];

pub(super) const HOOPING: &[Panel] = &[
    Panel {
        title: "Hooping",
        items: &[Large(f(Flag::HoopingMode)), Large(Cmd::AutoLayout)],
    },
    Panel {
        title: "Hoopings and Split Lines",
        items: &[
            Small(&[Cmd::AddHooping, Cmd::TurnHooping, Cmd::DeleteHooping]),
            Small(&[Cmd::DrawSplit, Cmd::DeleteSplit, Cmd::ClearHoopings]),
        ],
    },
    Panel {
        title: "Options",
        items: &[Custom(hooping_options, &[])],
    },
    Panel {
        title: "Output",
        items: &[Large(Cmd::PreviewHoopings), Large(Cmd::OutputHoopings)],
    },
];

pub(super) fn hooping_options(ui: &mut egui::Ui, st: &mut AppState, _s: &SelInfo) {
    crate::hooping::options(ui, st);
}

// ---- Simple mode (P10 Milestone 5): the tools of the patch and lettering tasks, large. ----

pub(super) const S_DIGITIZE: &[Panel] = &[
    Panel {
        title: "Select",
        items: &[Large(t(Tool::Select)), Large(t(Tool::Reshape))],
    },
    Panel {
        title: "Draw",
        items: &[
            Large(t(Tool::Line)),
            Large(t(Tool::Shape)),
            Large(t(Tool::Column)),
            Large(t(Tool::Rectangle)),
            Large(t(Tool::Ellipse)),
            Large(t(Tool::Applique)),
        ],
    },
    Panel {
        title: "Picture",
        items: &[Large(Cmd::PictureWizard), Large(Cmd::LoadPicture)],
    },
];

pub(super) const S_LETTERING: &[Panel] = &[
    Panel {
        title: "Lettering",
        items: &[Large(t(Tool::Lettering)), Large(Cmd::Fonts)],
    },
    Panel {
        title: "Personalize",
        items: &[Large(Cmd::Monogram), Large(Cmd::TeamNames), Large(Cmd::Kiosk)],
    },
];

pub(super) const S_EDIT: &[Panel] = &[
    Panel {
        title: "Clipboard",
        items: &[
            Large(Cmd::Paste),
            Large(Cmd::Copy),
            Large(Cmd::Cut),
            Large(Cmd::Duplicate),
            Large(Cmd::Delete),
        ],
    },
    Panel {
        title: "Arrange",
        items: &[
            Large(Cmd::RotateCw),
            Large(Cmd::MirrorH),
            Large(Cmd::CenterInHoop),
            Large(Cmd::SelectAll),
        ],
    },
];

pub(super) const S_VIEW: &[Panel] = &[
    Panel {
        title: "Show",
        items: &[Large(f(Flag::ThreadLook)), Large(f(Flag::Player))],
    },
    Panel {
        title: "Zoom",
        items: &[
            Large(Cmd::ZoomExtents),
            Large(Cmd::ZoomHoop),
            Large(Cmd::ZoomIn),
            Large(Cmd::ZoomOut),
        ],
    },
    Panel {
        title: "Mode",
        items: &[Large(f(Flag::SimpleMode))],
    },
];

pub(super) const S_DESIGN: &[Panel] = &[
    Panel {
        title: "Machine and Hoop",
        items: &[Custom(machine_hoop, &[]), Large(Cmd::AutoHoop)],
    },
    Panel {
        title: "Fabric",
        items: &[Large(Cmd::Fabric)],
    },
    Panel {
        title: "Output",
        items: &[Large(Cmd::Export), Large(Cmd::OutputDesign), Large(Cmd::SendToUsb)],
    },
];

pub(super) const HELP: &[Panel] = &[
    Panel {
        title: "Help",
        items: &[
            Large(Cmd::HelpContents),
            Large(Cmd::FindCommand),
            Large(Cmd::StartPage),
            Small(&[Cmd::KeyboardShortcuts, Cmd::HelpTopic("troubleshooting"), Cmd::About]),
        ],
    },
    Panel {
        title: "Mode",
        items: &[Large(f(Flag::SimpleMode))],
    },
    Panel {
        title: "Tutorials",
        items: &[
            Small(&[
                Cmd::HelpTopic("tutorial-patch"),
                Cmd::HelpTopic("tutorial-lettering"),
                Cmd::HelpTopic("tutorial-applique"),
            ]),
            Small(&[
                Cmd::HelpTopic("tutorial-towel"),
                Cmd::HelpTopic("tutorial-hooping"),
                Cmd::HelpTopic("tutorial-lace"),
            ]),
            Small(&[Cmd::HelpTopic("tutorial-output")]),
        ],
    },
];

/// All commands of the full ribbon (also in simple mode), with their tab
/// and panel title, in ribbon order (a command can be in two panels).
pub fn all_commands() -> Vec<(Cmd, RibbonTab, &'static str)> {
    // The quick-access buttons next to the File button first.
    let mut out: Vec<(Cmd, RibbonTab, &'static str)> = QUICK
        .iter()
        .map(|c| (*c, RibbonTab::Digitize, tr("Quick access")))
        .collect();
    for tab in RibbonTab::ALL {
        for p in tab.panels() {
            for item in p.items {
                match item {
                    Large(c) => out.push((*c, tab, p.title)),
                    Small(cs) | Custom(_, cs) => out.extend(cs.iter().map(|c| (*c, tab, p.title))),
                }
            }
        }
    }
    out
}

/// The commands next to the File button.
pub(super) const QUICK: [Cmd; 5] = [Cmd::New, Cmd::Open, Cmd::Save, Cmd::Undo, Cmd::Redo];
