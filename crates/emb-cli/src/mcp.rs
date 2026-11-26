//! The MCP server of `emb-cli mcp` (P22 Milestone 9, ADR 0176).
//!
//! The AI agent of the user (an MCP client) starts `emb-cli mcp` and talks
//! to it on standard input and output: JSON-RPC 2.0, one message on each
//! line (the stdio transport of MCP). The program has no AI in it and no
//! network code (ADR 0109): the agent of the user uses the functions of the
//! program through the tools of this server. The assistant of P10 stays
//! benched.
//!
//! The server keeps one design (a [`Document`] with its undo history).
//! Each change is one `Document::edit`, so one undo step; a change that
//! the document refuses (out of the range of a project) gives its reason.
//! A tool refuses an argument that it does not take, and names the ones
//! that it takes; a tool that only reads says so (`readOnlyHint`).
//! Coordinates are mm from the middle of the hoop, x to the right and y
//! down; angles are degrees.
//!
//! The protocol: the handshake `initialize` of MCP 2025-11-25 (and of the
//! versions before it), `server/discover` of MCP 2026-07-28, `ping`,
//! `tools/list` and `tools/call`. Nothing but messages goes to standard
//! output.

use std::collections::BTreeMap;
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};

use emb_geom::path::PathCmd;
use emb_geom::{BBox, Path as Outline, Point, Transform};
use emb_model::{Design, Document, HoopCatalog, HoopSelection, MachineCatalog, ObjectId, ObjectKind, Rgb, project};
use serde_json::{Map, Value, json};

use crate::{Input, plan, plan_options, read_input};

/// The versions of MCP with the handshake `initialize`, the newest first.
const VERSIONS: [&str; 4] = ["2025-11-25", "2025-06-18", "2025-03-26", "2024-11-05"];
/// The version of MCP with no handshake (`server/discover`).
const STATELESS: &str = "2026-07-28";

/// What the agent reads first.
const INSTRUCTIONS: &str = "ShokujoDX makes machine embroidery designs. This server keeps one \
design: start with new_design or open_design (set_design changes its machine, hoop and \
fabric), add objects with add_shape (SVG path text) and \
add_lettering, change them with set_settings, move_objects, scale_objects, rotate_objects and \
delete_objects, look at it with picture and check_design, and keep it with save_design or \
export. show opens it in the program. Each change is one undo step (undo, redo). Coordinates \
are mm from the middle of the hoop, x to the right and y down; angles are degrees.";

/// The most characters of the text of lettering that the tools take: a
/// text of 1 MB kept the server busy for more than 30 s (the audit of
/// 2026-11-26), and no hoop holds 2,000 letters.
const MAX_TEXT_CHARS: usize = 2_000;

/// Serves one agent: reads messages from `input` until it ends, and
/// writes the replies to `output`, one on each line. A line that is not
/// UTF-8 text gets an error, and the session goes on (its design is not
/// lost).
pub fn serve(mut input: impl BufRead, mut output: impl Write) -> std::io::Result<()> {
    let mut server = Session::new();
    let mut bytes = Vec::new();
    loop {
        bytes.clear();
        if input.read_until(b'\n', &mut bytes)? == 0 {
            return Ok(());
        }
        let reply = match std::str::from_utf8(&bytes) {
            Err(_) => Some(error(Value::Null, -32700, "the line is not UTF-8 text")),
            Ok(line) => {
                // A byte order mark at the start of the stream is not JSON.
                let line = line.trim_start_matches('\u{feff}');
                if line.trim().is_empty() {
                    continue;
                }
                match serde_json::from_str::<Value>(line) {
                    Ok(Value::Array(_)) => Some(error(Value::Null, -32600, "a batch of messages is not taken")),
                    Ok(message @ Value::Object(_)) => server.handle(&message),
                    Ok(_) => Some(error(Value::Null, -32600, "a message must be a JSON object")),
                    Err(e) => Some(error(Value::Null, -32700, &format!("not JSON: {e}"))),
                }
            }
        };
        if let Some(r) = reply {
            serde_json::to_writer(&mut output, &r)?;
            output.write_all(b"\n")?;
            output.flush()?;
        }
    }
}

fn error(id: Value, code: i64, message: &str) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message}})
}

fn reply(id: Value, result: Value) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "result": result})
}

fn server_info() -> Value {
    json!({"name": "shokujodx", "title": "ShokujoDX", "version": env!("CARGO_PKG_VERSION")})
}

/// The design of the agent, and its project file.
pub struct Session {
    doc: Document,
    path: Option<PathBuf>,
    /// The file in the temp folder that `show` writes for a design with no
    /// file (the design stays not saved).
    shown: Option<PathBuf>,
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}

impl Session {
    pub fn new() -> Session {
        Session {
            doc: Document::new(Design::new("Design")),
            path: None,
            shown: None,
        }
    }

    /// The reply to one message (`None` for a notification or a reply of
    /// the client).
    pub fn handle(&mut self, m: &Value) -> Option<Value> {
        let Some(method) = m.get("method").and_then(Value::as_str) else {
            // A reply of the client (a result or an error) gets no reply; a
            // request with no method gets an error, so the client does not
            // wait for ever.
            return match m.get("id") {
                Some(id) if m.get("result").is_none() && m.get("error").is_none() => {
                    Some(error(id.clone(), -32600, "a request needs a method (text)"))
                }
                _ => None,
            };
        };
        // A message with no id is a notification: no reply.
        let id = m.get("id").cloned()?;
        let params = m.get("params").cloned().unwrap_or(json!({}));
        let result = match method {
            "initialize" => {
                let asked = params.get("protocolVersion").and_then(Value::as_str).unwrap_or("");
                let version = VERSIONS.iter().find(|v| **v == asked).copied().unwrap_or(VERSIONS[0]);
                json!({
                    "protocolVersion": version,
                    "capabilities": {"tools": {"listChanged": false}},
                    "serverInfo": server_info(),
                    "instructions": INSTRUCTIONS,
                })
            }
            "server/discover" => {
                let mut supported = vec![STATELESS];
                supported.extend(VERSIONS);
                json!({
                    "resultType": "complete",
                    "supportedVersions": supported,
                    "capabilities": {"tools": {}},
                    "_meta": {"io.modelcontextprotocol/serverInfo": server_info()},
                    "instructions": INSTRUCTIONS,
                })
            }
            "ping" => json!({}),
            "tools/list" => json!({
                "resultType": "complete",
                "tools": TOOLS.iter().map(Tool::listing).collect::<Vec<_>>(),
            }),
            "tools/call" => {
                let name = params.get("name").and_then(Value::as_str).unwrap_or("");
                let Some(tool) = TOOLS.iter().find(|t| t.name == name) else {
                    return Some(error(id, -32602, &format!("Unknown tool: {name}")));
                };
                let args = match params.get("arguments") {
                    None | Some(Value::Null) => Map::new(),
                    Some(Value::Object(a)) => a.clone(),
                    Some(_) => return Some(error(id, -32602, "the arguments must be an object")),
                };
                self.call(tool, &args)
            }
            _ => return Some(error(id, -32601, &format!("Method not found: {method}"))),
        };
        Some(reply(id, result))
    }

    /// Calls `tool` with `args`: the result of the tool, or its error as a
    /// result with `isError` (the agent can correct its call).
    fn call(&mut self, tool: &Tool, args: &Map<String, Value>) -> Value {
        // A fault (a panic) in a tool gives an error, and the session and
        // its design stay (the audit of 2026-11-26: a color with a letter
        // that is not ASCII stopped the server).
        let out = check_args(tool, args).and_then(|()| {
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| (tool.run)(self, &Args(args))))
                .unwrap_or_else(|_| Err(format!("{} stopped with a fault of the program.", tool.name)))
        });
        match out {
            Ok(out) => {
                let mut content = vec![json!({"type": "text", "text": out.value.to_string()})];
                if let Some(png) = &out.png {
                    content.push(json!({"type": "image", "data": base64(png), "mimeType": "image/png"}));
                }
                json!({
                    "resultType": "complete",
                    "content": content,
                    "structuredContent": out.value,
                    "isError": false,
                })
            }
            Err(e) => json!({
                "resultType": "complete",
                "content": [{"type": "text", "text": e}],
                "isError": true,
            }),
        }
    }

    pub fn design(&self) -> &Design {
        self.doc.design()
    }

    /// One change of the design: one undo step. A change that the document
    /// refuses gives its reason.
    fn change<R>(&mut self, label: &str, f: impl FnOnce(&mut Design) -> R) -> Result<R, String> {
        let r = self.doc.edit(label, f);
        match self.doc.take_refused() {
            Some(why) => Err(format!("The change was refused: {why}.")),
            None => Ok(r),
        }
    }

    /// The design can go: it has no changes that are not saved, or the
    /// agent says to discard them.
    fn may_replace(&self, args: &Args) -> Result<(), String> {
        let discard = args.flag("discard_changes")?.unwrap_or(false);
        if self.doc.is_modified() && !self.design().objects().is_empty() && !discard {
            return Err(
                "The design has changes that are not saved. Use save_design first, or discard_changes: true.".into(),
            );
        }
        Ok(())
    }

    /// The design now: its name, file, machine, hoop, size, threads and
    /// stitch numbers.
    fn summary(&self) -> Value {
        let d = self.design();
        let size = d
            .view_extents()
            .map(|b| json!({"width": round(b.width()), "height": round(b.height())}));
        let hoop = match &d.hoop.selection {
            HoopSelection::Preset { id } => HoopCatalog::builtin()
                .get(id)
                .map(|h| json!({"id": id, "name": h.name, "width": h.width_mm, "height": h.height_mm})),
            HoopSelection::OpenField => None,
        };
        json!({
            "name": d.name,
            "file": self.path.as_ref().map(|p| p.display().to_string()),
            "saved": !self.doc.is_modified(),
            "machine": d.machine,
            "hoop": hoop,
            "fabric": d.fabric,
            "size_mm": size,
            "objects": d.objects().len(),
            "threads": d.threads.iter().map(|t| json!({"id": t.id.0, "name": t.name, "color": t.color.to_hex()})).collect::<Vec<_>>(),
            "stitches": numbers(d),
            "undo_steps": self.doc.undo_len(),
            "undo": self.doc.undo_label(),
            "redo": self.doc.redo_label(),
        })
    }
}

/// A value of a tool, and a picture.
struct Out {
    value: Value,
    png: Option<Vec<u8>>,
}

fn out(value: Value) -> Result<Out, String> {
    Ok(Out { value, png: None })
}

/// The type of an argument.
#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Number,
    Text,
    Flag,
    Ids,
    Texts,
    Object,
}

struct Param {
    name: &'static str,
    kind: Kind,
    required: bool,
    about: &'static str,
}

const fn p(name: &'static str, kind: Kind, required: bool, about: &'static str) -> Param {
    Param {
        name,
        kind,
        required,
        about,
    }
}

/// A tool: a function of the program.
struct Tool {
    name: &'static str,
    title: &'static str,
    about: &'static str,
    params: &'static [Param],
    /// It only reads (the design and files); it changes nothing.
    reads: bool,
    /// It can take away work: a design that is not saved, objects, a file.
    destroys: bool,
    run: fn(&mut Session, &Args) -> Result<Out, String>,
}

impl Tool {
    fn listing(&self) -> Value {
        let mut properties = Map::new();
        for p in self.params {
            let mut s = match p.kind {
                Kind::Number => json!({"type": "number"}),
                Kind::Text => json!({"type": "string"}),
                Kind::Flag => json!({"type": "boolean"}),
                Kind::Ids => json!({"type": "array", "items": {"type": "integer"}}),
                Kind::Texts => json!({"type": "array", "items": {"type": "string"}}),
                Kind::Object => json!({"type": "object"}),
            };
            s["description"] = json!(p.about);
            properties.insert(p.name.into(), s);
        }
        let required: Vec<&str> = self.params.iter().filter(|p| p.required).map(|p| p.name).collect();
        let about = if self.reads {
            format!("{} It only reads.", self.about)
        } else {
            self.about.to_string()
        };
        json!({
            "name": self.name,
            "title": self.title,
            "description": about,
            "inputSchema": {
                "type": "object",
                "properties": properties,
                "required": required,
                "additionalProperties": false,
            },
            "annotations": {
                "title": self.title,
                "readOnlyHint": self.reads,
                "destructiveHint": self.destroys,
                "idempotentHint": self.reads,
                "openWorldHint": false,
            },
        })
    }
}

/// A tool refuses an argument that it does not take (and names the ones
/// that it takes), an argument that it needs and does not get, and an
/// argument of the wrong type.
fn check_args(tool: &Tool, args: &Map<String, Value>) -> Result<(), String> {
    let names = || tool.params.iter().map(|p| p.name).collect::<Vec<_>>().join(", ");
    for k in args.keys() {
        if !tool.params.iter().any(|p| p.name == k) {
            return Err(if tool.params.is_empty() {
                format!("{} takes no arguments (it got `{k}`).", tool.name)
            } else {
                format!("{} takes no argument `{k}`. It takes: {}.", tool.name, names())
            });
        }
    }
    let a = Args(args);
    for p in tool.params {
        let present = args.get(p.name).is_some_and(|v| !v.is_null());
        if p.required && !present {
            return Err(format!("{} needs `{}`: {}", tool.name, p.name, p.about));
        }
        if !present {
            continue;
        }
        match p.kind {
            Kind::Number => a.number(p.name).map(|_| ())?,
            Kind::Text => a.text(p.name).map(|_| ())?,
            Kind::Flag => a.flag(p.name).map(|_| ())?,
            Kind::Ids => a.ids(p.name).map(|_| ())?,
            Kind::Texts => a.texts(p.name).map(|_| ())?,
            Kind::Object => a.object(p.name).map(|_| ())?,
        }
    }
    Ok(())
}

/// The arguments of a call.
struct Args<'a>(&'a Map<String, Value>);

impl Args<'_> {
    fn get(&self, k: &str) -> Option<&Value> {
        self.0.get(k).filter(|v| !v.is_null())
    }

    fn number(&self, k: &str) -> Result<Option<f64>, String> {
        match self.get(k) {
            None => Ok(None),
            Some(v) => v
                .as_f64()
                .filter(|x| x.is_finite())
                .map(Some)
                .ok_or_else(|| format!("`{k}` must be a number")),
        }
    }

    fn need_number(&self, k: &str) -> Result<f64, String> {
        self.number(k)?.ok_or_else(|| format!("`{k}` is needed"))
    }

    fn text(&self, k: &str) -> Result<Option<&str>, String> {
        match self.get(k) {
            None => Ok(None),
            Some(v) => v.as_str().map(Some).ok_or_else(|| format!("`{k}` must be text")),
        }
    }

    fn need_text(&self, k: &str) -> Result<&str, String> {
        self.text(k)?.ok_or_else(|| format!("`{k}` is needed"))
    }

    fn flag(&self, k: &str) -> Result<Option<bool>, String> {
        match self.get(k) {
            None => Ok(None),
            Some(v) => v
                .as_bool()
                .map(Some)
                .ok_or_else(|| format!("`{k}` must be true or false")),
        }
    }

    fn ids(&self, k: &str) -> Result<Vec<ObjectId>, String> {
        let bad = || format!("`{k}` must be a list of object ids (whole numbers; list_objects gives them)");
        match self.get(k) {
            None => Ok(Vec::new()),
            Some(Value::Array(a)) => a.iter().map(|v| v.as_u64().map(ObjectId).ok_or_else(bad)).collect(),
            Some(_) => Err(bad()),
        }
    }

    fn texts(&self, k: &str) -> Result<Vec<String>, String> {
        let bad = || format!("`{k}` must be a list of texts");
        match self.get(k) {
            None => Ok(Vec::new()),
            Some(Value::Array(a)) => a
                .iter()
                .map(|v| v.as_str().map(str::to_string).ok_or_else(bad))
                .collect(),
            Some(_) => Err(bad()),
        }
    }

    fn object(&self, k: &str) -> Result<Option<&Map<String, Value>>, String> {
        match self.get(k) {
            None => Ok(None),
            Some(v) => v
                .as_object()
                .map(Some)
                .ok_or_else(|| format!("`{k}` must be an object")),
        }
    }
}

/// The ids, all of them in the design.
fn objects_of(s: &Session, a: &Args) -> Result<Vec<ObjectId>, String> {
    let mut ids = a.ids("ids")?;
    // An id two times moves, scales or turns its object one time.
    let mut seen = std::collections::HashSet::new();
    ids.retain(|id| seen.insert(*id));
    if ids.is_empty() {
        return Err("`ids` is empty: give the ids of the objects (list_objects)".into());
    }
    if let Some(id) = ids.iter().find(|id| s.design().object(**id).is_none()) {
        return Err(format!(
            "the design has no object {} (list_objects gives the ids)",
            id.0
        ));
    }
    Ok(ids)
}

static TOOLS: [Tool; 26] = [
    Tool {
        name: "new_design",
        title: "New design",
        about: "Starts a new, empty design in place of the design of the session.",
        params: &[
            p("name", Kind::Text, false, "The name of the design."),
            p(
                "machine",
                Kind::Text,
                false,
                "The id of the machine (list_catalog machines).",
            ),
            p("hoop", Kind::Text, false, "The id of the hoop (list_catalog hoops)."),
            p(
                "fabric",
                Kind::Text,
                false,
                "The id of the fabric (list_catalog fabrics): the settings of new objects suit it.",
            ),
            p(
                "discard_changes",
                Kind::Flag,
                false,
                "Discard the changes of the design now that are not saved.",
            ),
        ],
        reads: false,
        destroys: true,
        run: new_design,
    },
    Tool {
        name: "set_design",
        title: "Design settings",
        about: "Changes the name, machine, hoop or fabric of the design, as one undo step. A new fabric changes the settings of the satin, fill and lettering objects for it, and of new objects.",
        params: &[
            p("name", Kind::Text, false, "The name of the design."),
            p(
                "machine",
                Kind::Text,
                false,
                "The id of the machine (list_catalog machines).",
            ),
            p("hoop", Kind::Text, false, "The id of the hoop (list_catalog hoops)."),
            p(
                "fabric",
                Kind::Text,
                false,
                "The id of the fabric (list_catalog fabrics), or \"none\".",
            ),
        ],
        reads: false,
        destroys: false,
        run: set_design,
    },
    Tool {
        name: "open_design",
        title: "Open design",
        about: "Opens a design: a project (.embproj), a machine file (as stitch objects) or an SVG file.",
        params: &[
            p("path", Kind::Text, true, "The file."),
            p(
                "discard_changes",
                Kind::Flag,
                false,
                "Discard the changes of the design now that are not saved.",
            ),
        ],
        reads: false,
        destroys: true,
        run: open_design,
    },
    Tool {
        name: "save_design",
        title: "Save design",
        about: "Saves the design as a project file (.embproj).",
        params: &[p(
            "path",
            Kind::Text,
            false,
            "The project file. Not needed if the design has a file.",
        )],
        reads: false,
        destroys: true,
        run: save_design,
    },
    Tool {
        name: "design_info",
        title: "Design information",
        about: "The design: its name, file, machine, hoop (its field is round the middle, 0, 0), fabric, size and box, if it fits the hoop, threads, stitches, color changes, trims, the travel that shows, and the undo steps.",
        params: &[],
        reads: true,
        destroys: false,
        run: design_info,
    },
    Tool {
        name: "list_objects",
        title: "List objects",
        about: "The objects in sew order: id, kind, thread, box (mm: x and y of the top left corner, width, height), if the shape fits the hoop, and settings.",
        params: &[p("ids", Kind::Ids, false, "Only these objects.")],
        reads: true,
        destroys: false,
        run: list_objects,
    },
    Tool {
        name: "add_shape",
        title: "Add a shape",
        about: "Adds an object from SVG path text (M, L, H, V, C, S, Q, T, A, Z; mm): a run along the path, a fill of its closed parts (the first part the outline, the others holes), a satin column with the path as its middle line, or a vector shape with no stitches. Coordinates are mm from the middle of the hoop, x to the right and y down.",
        params: &[
            p(
                "svg_path",
                Kind::Text,
                true,
                "SVG path data in mm, for example \"M -30 0 A 30 30 0 1 0 30 0 A 30 30 0 1 0 -30 0 Z\".",
            ),
            p("stitch", Kind::Text, true, "run, fill, satin or vector."),
            p(
                "width_mm",
                Kind::Number,
                false,
                "The full width of a satin column, mm: half of it on each side of the path (4 if not given).",
            ),
            p(
                "color",
                Kind::Text,
                false,
                "The thread color, #rrggbb. The first thread if not given.",
            ),
        ],
        reads: false,
        destroys: false,
        run: add_shape,
    },
    Tool {
        name: "add_lettering",
        title: "Add lettering",
        about: "Adds lettering in a bundled embroidery font, with its middle at x, y.",
        params: &[
            p("text", Kind::Text, true, "The text (a new line starts a line)."),
            p(
                "font",
                Kind::Text,
                false,
                "A bundled embroidery font (list_catalog fonts). Block Sans if not given.",
            ),
            p(
                "height_mm",
                Kind::Number,
                false,
                "The height of the capital letters (15 if not given).",
            ),
            p(
                "x",
                Kind::Number,
                false,
                "The middle of the lettering, mm (0 if not given).",
            ),
            p(
                "y",
                Kind::Number,
                false,
                "The middle of the lettering, mm (0 if not given).",
            ),
            p(
                "color",
                Kind::Text,
                false,
                "The thread color, #rrggbb. The first thread if not given.",
            ),
        ],
        reads: false,
        destroys: false,
        run: add_lettering,
    },
    Tool {
        name: "set_settings",
        title: "Change settings",
        about: "Changes the settings of objects: the names that list_objects shows for each object, for example {\"spacing_mm\": 0.4, \"angle_deg\": 45} for a fill. Lettering also takes text, height_mm, font (the name of a font) and its other settings, and keeps the middle of its box. A wrong value gives an error that names the values that are allowed.",
        params: &[
            p("ids", Kind::Ids, true, "The objects."),
            p("settings", Kind::Object, true, "The settings to change, by name."),
        ],
        reads: false,
        destroys: false,
        run: set_settings,
    },
    Tool {
        name: "set_thread",
        title: "Change thread",
        about: "Gives objects the thread of a color (a thread of the design with that color, else a new thread).",
        params: &[
            p("ids", Kind::Ids, true, "The objects."),
            p("color", Kind::Text, true, "The color, #rrggbb."),
            p("name", Kind::Text, false, "The name of a new thread."),
        ],
        reads: false,
        destroys: false,
        run: set_thread,
    },
    Tool {
        name: "move_objects",
        title: "Move",
        about: "Moves objects by dx and dy, or puts the middle of their box at x and y (0, 0 is the middle of the hoop).",
        params: &[
            p("ids", Kind::Ids, true, "The objects."),
            p("dx", Kind::Number, false, "To the right, mm."),
            p("dy", Kind::Number, false, "Down, mm."),
            p("x", Kind::Number, false, "The new middle of the box, mm (not with dx)."),
            p("y", Kind::Number, false, "The new middle of the box, mm (not with dy)."),
        ],
        reads: false,
        destroys: false,
        run: move_objects,
    },
    Tool {
        name: "scale_objects",
        title: "Scale",
        about: "Scales objects about the middle of their box: factor for both directions, or sx and sy.",
        params: &[
            p("ids", Kind::Ids, true, "The objects."),
            p("factor", Kind::Number, false, "The factor, for example 1.5."),
            p("sx", Kind::Number, false, "The factor across."),
            p("sy", Kind::Number, false, "The factor down."),
        ],
        reads: false,
        destroys: false,
        run: scale_objects,
    },
    Tool {
        name: "rotate_objects",
        title: "Rotate",
        about: "Rotates objects about the middle of their box, clockwise on the screen.",
        params: &[
            p("ids", Kind::Ids, true, "The objects."),
            p("degrees", Kind::Number, true, "The angle."),
        ],
        reads: false,
        destroys: false,
        run: rotate_objects,
    },
    Tool {
        name: "delete_objects",
        title: "Delete",
        about: "Deletes objects.",
        params: &[p("ids", Kind::Ids, true, "The objects.")],
        reads: false,
        destroys: true,
        run: delete_objects,
    },
    Tool {
        name: "undo",
        title: "Undo",
        about: "Takes back the last change.",
        params: &[],
        reads: false,
        destroys: false,
        run: undo,
    },
    Tool {
        name: "redo",
        title: "Redo",
        about: "Makes the last change that undo took back again.",
        params: &[],
        reads: false,
        destroys: false,
        run: redo,
    },
    Tool {
        name: "run_script",
        title: "Run a script",
        about: "Runs a script of the program (Rhai; the help topic Scripts has its functions) on the design. Its change is one undo step.",
        params: &[
            p("source", Kind::Text, true, "The script."),
            p("ids", Kind::Ids, false, "The selection that the script sees."),
            p(
                "inputs",
                Kind::Object,
                false,
                "The inputs of the script, by name (numbers, true or false, texts).",
            ),
        ],
        reads: false,
        destroys: false,
        run: run_script,
    },
    Tool {
        name: "check_design",
        title: "Check",
        about: "The warnings of the design: stitches outside the field of the hoop, the warnings of the stitch plan (density, sizes), travel that shows, loose parts of lace, the faults of stitch objects.",
        params: &[],
        reads: true,
        destroys: false,
        run: check_design,
    },
    Tool {
        name: "picture",
        title: "Picture",
        about: "A picture of the design with the 3D thread look (PNG), and its file if a path is given.",
        params: &[
            p("path", Kind::Text, false, "A PNG file to write."),
            p(
                "px_per_mm",
                Kind::Number,
                false,
                "Pixels for each mm (4 if not given; 0.5 to 20, and at most 2,000 pixels on a side).",
            ),
            p(
                "background",
                Kind::Text,
                false,
                "A plain color behind the design, #rrggbb, or \"none\" (clear). If not given: the fabric of the design (its weave in its color, else white).",
            ),
            p(
                "flat",
                Kind::Flag,
                false,
                "The flat look (lines) in place of the 3D look.",
            ),
        ],
        reads: false,
        destroys: true,
        run: picture,
    },
    Tool {
        name: "export",
        title: "Export",
        about: "Writes the output files of the design to a folder: machine files and, if asked, the worksheet, the approval sheet and the picture. Also the project file, which becomes the file of the design. PES and PEC files keep each thread as the nearest of the 64 colors of the Brother palette.",
        params: &[
            p("folder", Kind::Text, true, "The folder."),
            p(
                "formats",
                Kind::Texts,
                false,
                "Machine formats by extension, for example [\"pes\", \"dst\"]. The format of the machine if not given.",
            ),
            p("worksheet", Kind::Flag, false, "The production worksheet (PDF)."),
            p("approval", Kind::Flag, false, "The approval sheet (PDF)."),
            p("picture", Kind::Flag, false, "The picture (PNG)."),
        ],
        reads: false,
        destroys: true,
        run: export,
    },
    Tool {
        name: "test_swatches",
        title: "Test swatches",
        about: "Writes a new project with copies of objects, one for each value of a setting in equal steps, each with its value under it (sew it to find the setting for a fabric). The design of the session does not change.",
        params: &[
            p("ids", Kind::Ids, true, "The objects to copy."),
            p(
                "setting",
                Kind::Text,
                true,
                "spacing (the density: mm from one stitch to the next), stitch length, pull compensation, width, angle, underlay, lean or band overlap.",
            ),
            p(
                "from",
                Kind::Number,
                false,
                "The value of the first copy: mm, or degrees for angle and lean (not for underlay).",
            ),
            p(
                "to",
                Kind::Number,
                false,
                "The value of the last copy (not for underlay).",
            ),
            p(
                "copies",
                Kind::Number,
                false,
                "The number of copies (5 if not given, at most 24).",
            ),
            p("path", Kind::Text, true, "The project file to write."),
        ],
        reads: false,
        destroys: true,
        run: test_swatches,
    },
    Tool {
        name: "quicker_to_sew",
        title: "Quicker to Sew",
        about: "Makes the design quicker to sew: merges similar threads, removes small details, joins color blocks, and sets the trim distance. Gives the numbers before and after and what changes (changed; with dry_run nothing changes, and would_change says if it would).",
        params: &[
            p(
                "merge_threads",
                Kind::Number,
                false,
                "Merge threads whose colors differ by this or less (CIE76; 5 if not given; 0: only the same color).",
            ),
            p(
                "small_details_mm",
                Kind::Number,
                false,
                "Remove fills, satin columns and lines smaller than this, mm (1 if not given; 0: none).",
            ),
            p(
                "fewer_color_changes",
                Kind::Flag,
                false,
                "Join color blocks (true if not given).",
            ),
            p(
                "gap_mm",
                Kind::Number,
                false,
                "Sew across gaps up to this, mm (3 if not given). Lettering has its own setting for the gaps between letters (trim_over_mm).",
            ),
            p("dry_run", Kind::Flag, false, "Only give the numbers; change nothing."),
        ],
        reads: false,
        destroys: true,
        run: quicker_to_sew,
    },
    Tool {
        name: "check_machine_file",
        title: "Check a machine file",
        about: "Checks the stitches of a machine file (or of the stitch objects of a project): short and long stitches, color changes to the same thread, long jumps with no trim, short pieces, no lock stitches, travel that shows. Also its size, stitches, colors, trims and threads (the colors that the file keeps).",
        params: &[
            p("path", Kind::Text, true, "The file."),
            p(
                "machine",
                Kind::Text,
                false,
                "The id of the machine for the stitch lengths.",
            ),
            p(
                "hoop",
                Kind::Text,
                false,
                "The id of a hoop: the result says if the stitches fit its field.",
            ),
        ],
        reads: true,
        destroys: false,
        run: check_machine_file,
    },
    Tool {
        name: "fix_machine_file",
        title: "Fix a machine file",
        about: "Fixes the stitches of a machine file and writes the result: joins color blocks of the same thread, removes short stitches, splits long stitches, trims long jumps, adds lock stitches. The other needle points stay.",
        params: &[
            p("path", Kind::Text, true, "The file."),
            p(
                "out",
                Kind::Text,
                true,
                "The file to write (its extension gives the format).",
            ),
            p(
                "fixes",
                Kind::Texts,
                false,
                "Some of join, short, long, jumps, locks (all if not given).",
            ),
            p(
                "machine",
                Kind::Text,
                false,
                "The id of the machine for the stitch lengths.",
            ),
        ],
        reads: false,
        destroys: true,
        run: fix_machine_file,
    },
    Tool {
        name: "show",
        title: "Show in the program",
        about: "Saves the design (to its file, or to a file in the folder for temporary files) and opens it in ShokujoDX: in a tab of the program that runs, else in a new program.",
        params: &[],
        reads: false,
        destroys: false,
        run: show,
    },
    Tool {
        name: "list_catalog",
        title: "List names",
        about: "The names to use in other tools: fonts (kind, size range in mm, quality score), machines (brand, formats, hoops), hoops, fabrics or swatch settings.",
        params: &[p(
            "what",
            Kind::Text,
            true,
            "fonts, machines, hoops, fabrics or settings.",
        )],
        reads: true,
        destroys: false,
        run: list_catalog,
    },
];

// --- The tools -----------------------------------------------------------

fn new_design(s: &mut Session, a: &Args) -> Result<Out, String> {
    s.may_replace(a)?;
    let mut d = Design::new(a.text("name")?.unwrap_or("Design"));
    if let Some(m) = a.text("machine")? {
        if MachineCatalog::builtin().get(m).is_none() {
            return Err(format!("no machine {m:?} (list_catalog machines)"));
        }
        d.machine = Some(m.to_string());
    }
    if let Some(h) = a.text("hoop")? {
        if HoopCatalog::builtin().get(h).is_none() {
            return Err(format!("no hoop {h:?} (list_catalog hoops)"));
        }
        d.hoop.selection = HoopSelection::Preset { id: h.to_string() };
    }
    if let Some(f) = a.text("fabric")? {
        let fabrics = emb_model::FabricCatalog::builtin();
        if fabrics.get(f).is_none() {
            return Err(format!("no fabric {f:?} (list_catalog fabrics)"));
        }
        d.set_fabric(&fabrics, Some(f), false);
    }
    s.doc = Document::new(d);
    s.path = None;
    out(s.summary())
}

fn set_design(s: &mut Session, a: &Args) -> Result<Out, String> {
    let name = a.text("name")?.map(str::to_string);
    let machine = a.text("machine")?.map(str::to_string);
    let hoop = a.text("hoop")?.map(str::to_string);
    let fabrics = emb_model::FabricCatalog::builtin();
    let fabric = match a.text("fabric")? {
        None => None,
        Some(f) if f.eq_ignore_ascii_case("none") => Some(None),
        Some(f) if fabrics.get(f).is_some() => Some(Some(f.to_string())),
        Some(f) => return Err(format!("no fabric {f:?} (list_catalog fabrics)")),
    };
    if name.is_none() && machine.is_none() && hoop.is_none() && fabric.is_none() {
        return Err("give one or more of name, machine, hoop, fabric".into());
    }
    if let Some(m) = &machine
        && MachineCatalog::builtin().get(m).is_none()
    {
        return Err(format!("no machine {m:?} (list_catalog machines)"));
    }
    if let Some(h) = &hoop
        && HoopCatalog::builtin().get(h).is_none()
    {
        return Err(format!("no hoop {h:?} (list_catalog hoops)"));
    }
    let changed = s.change("Design settings", |d| {
        if let Some(n) = name {
            d.name = n;
        }
        if let Some(m) = machine {
            d.machine = Some(m);
        }
        if let Some(h) = hoop {
            d.hoop.selection = HoopSelection::Preset { id: h };
        }
        fabric.map(|f| d.set_fabric(&fabrics, f.as_deref(), true).len())
    })?;
    let mut v = s.summary();
    if let Some(n) = changed {
        v["objects_changed_for_the_fabric"] = json!(n);
    }
    out(v)
}

fn open_design(s: &mut Session, a: &Args) -> Result<Out, String> {
    s.may_replace(a)?;
    let path = PathBuf::from(a.need_text("path")?);
    let d = match read_input(&path)? {
        Input::Design(d) => *d,
        Input::Stitches(st) => emb_model::design_from_stitches(&st, &crate::stem(&path)),
    };
    let is_project = path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case(project::FILE_EXTENSION));
    s.doc = Document::new(d);
    s.path = is_project.then_some(path);
    out(s.summary())
}

fn save_design(s: &mut Session, a: &Args) -> Result<Out, String> {
    let path = match a.text("path")? {
        Some(p) => {
            let mut p = PathBuf::from(p);
            match p.extension().map(|e| e.to_string_lossy().to_lowercase()) {
                None => {
                    p.set_extension(project::FILE_EXTENSION);
                }
                Some(e) if e == project::FILE_EXTENSION => {}
                Some(e) => {
                    return Err(format!(
                        "a project file ends in .{}; export writes .{e} files",
                        project::FILE_EXTENSION
                    ));
                }
            }
            p
        }
        None => s.path.clone().ok_or("the design has no file yet: give `path`")?,
    };
    project::save(s.design(), &path).map_err(|e| format!("{}: {e}", path.display()))?;
    s.doc.mark_saved();
    s.path = Some(path.clone());
    out(json!({"saved": path.display().to_string()}))
}

fn design_info(s: &mut Session, _: &Args) -> Result<Out, String> {
    let mut v = s.summary();
    let d = s.design();
    v["box_mm"] = json!(boxes(d, &d.objects().iter().map(|o| o.id).collect::<Vec<_>>()).map(box_value));
    if let Some(h) = hoop_of(d) {
        let p = plan(d, &plan_options(d, None).unwrap_or_default());
        v["fits_hoop"] = json!(outside_hoop(d, &h, &p).is_empty());
    }
    out(v)
}

fn list_objects(s: &mut Session, a: &Args) -> Result<Out, String> {
    let only = a.ids("ids")?;
    let d = s.design();
    let list: Vec<Value> = d
        .objects()
        .iter()
        .filter(|o| only.is_empty() || only.contains(&o.id))
        .map(|o| {
            let b = boxes(d, &[o.id]);
            let thread = d.thread(o.thread);
            let mut v = json!({
                "id": o.id.0,
                "fits_hoop": shape_fits_hoop(d, o.id),
                "kind": o.type_name(),
                "thread": {"id": o.thread.0, "color": thread.map(|t| t.color.to_hex())},
                "box_mm": b.map(box_value),
                "settings": shown_settings(&o.kind),
            });
            if let Some(n) = &o.name {
                v["name"] = json!(n);
            }
            if o.locked {
                v["locked"] = json!(true);
            }
            v
        })
        .collect();
    out(json!({"objects": list}))
}

fn add_shape(s: &mut Session, a: &Args) -> Result<Out, String> {
    let stitch = a.need_text("stitch")?.to_lowercase();
    let svg = a.need_text("svg_path")?;
    let color = a.text("color")?.map(parse_color).transpose()?;
    let width = a.number("width_mm")?.unwrap_or(4.0);
    let current = s.design().current.clone();
    let kind = match stitch.as_str() {
        "vector" => {
            let mut kinds = svg_kinds(svg)?;
            if kinds.len() > 1 {
                return Err("the path text gives more than one shape".into());
            }
            kinds.remove(0)
        }
        "run" => ObjectKind::Running {
            path: svg_outline(svg)?,
            params: current.running,
        },
        "satin" => {
            if width <= 0.0 {
                return Err("`width_mm` must be more than 0".into());
            }
            ObjectKind::Satin {
                geometry: emb_model::SatinGeometry::from_centerline(&svg_outline(svg)?, width),
                params: current.satin,
            }
        }
        "fill" => {
            let mut parts = subpaths(&svg_outline(svg)?).into_iter();
            let outer = parts.next().ok_or("the path text gives no shape")?;
            ObjectKind::Fill {
                outer,
                holes: parts.collect(),
                params: current.fill,
            }
        }
        other => return Err(format!("`stitch` is run, fill, satin or vector, not {other:?}")),
    };
    let id = s.change("Add shape", |d| {
        let t = thread_of(d, color);
        d.add_object(t, kind)
    })?;
    out(json!({"added": id.0, "object": object_value(s.design(), id)}))
}

fn add_lettering(s: &mut Session, a: &Args) -> Result<Out, String> {
    let text = a.need_text("text")?;
    if text.trim().is_empty() {
        return Err("`text` is empty".into());
    }
    if text.chars().count() > MAX_TEXT_CHARS {
        return Err(format!("`text` has more than {MAX_TEXT_CHARS} characters"));
    }
    let family = a.text("font")?.unwrap_or(emb_letter::bundled::DEFAULT_EMBROIDERY);
    let font = emb_letter::bundled::embroidery(family)
        .ok_or_else(|| format!("no bundled embroidery font {family:?} (list_catalog fonts)"))?;
    let height = a.number("height_mm")?.unwrap_or(15.0);
    if height <= 0.0 {
        return Err("`height_mm` must be more than 0".into());
    }
    let at = Point::new(a.number("x")?.unwrap_or(0.0), a.number("y")?.unwrap_or(0.0));
    let color = a.text("color")?.map(parse_color).transpose()?;
    let mut l = emb_model::Lettering::new(text, emb_letter::LetterSource::font_ref(&font), height, Point::ZERO);
    emb_letter::relayout(&mut l, &font);
    let mut kind = ObjectKind::Lettering(Box::new(l));
    if let Some(b) = kind_box(&kind) {
        let c = b.center();
        kind.transform(&Transform::translate(at.x - c.x, at.y - c.y));
    }
    let id = s.change("Add lettering", |d| {
        let mut kind = kind;
        d.fabric_change(&emb_model::FabricCatalog::builtin()).object(&mut kind);
        let t = thread_of(d, color);
        d.add_object(t, kind)
    })?;
    out(json!({"added": id.0, "object": object_value(s.design(), id)}))
}

fn set_settings(s: &mut Session, a: &Args) -> Result<Out, String> {
    let ids = objects_of(s, a)?;
    let patch = a.object("settings")?.cloned().unwrap_or_default();
    if patch.is_empty() {
        return Err("`settings` is empty".into());
    }
    // All the new settings first: an error changes nothing.
    let mut new = Vec::new();
    for id in &ids {
        let o = s.design().object(*id).ok_or("no object")?;
        new.push((
            *id,
            with_settings(&o.kind, &patch).map_err(|e| format!("object {}: {e}", id.0))?,
        ));
    }
    s.change("Change settings", |d| {
        for (id, kind) in new {
            // Lettering that is laid out again keeps the middle of its box.
            let keep = matches!(kind, ObjectKind::Lettering(_))
                .then(|| boxes(d, &[id]).map(|b| b.center()))
                .flatten();
            if let Some(o) = d.object_mut(id) {
                o.kind = kind;
            }
            if let (Some(was), Some(now)) = (keep, boxes(d, &[id]).map(|b| b.center()))
                && was.distance(now) > 1e-9
            {
                d.transform_objects(&[id], &Transform::translate(was.x - now.x, was.y - now.y));
            }
        }
    })?;
    let changed: Vec<Value> = ids.iter().map(|id| object_value(s.design(), *id)).collect();
    out(json!({"changed": changed}))
}

fn set_thread(s: &mut Session, a: &Args) -> Result<Out, String> {
    let ids = objects_of(s, a)?;
    let color = parse_color(a.need_text("color")?)?;
    let name = a.text("name")?.map(str::to_string);
    let t = s.change("Change thread", |d| {
        let had = d.threads.iter().any(|t| t.color == color);
        let t = d.thread_for_color(color);
        if !had
            && let Some(n) = name
            && let Some(th) = d.threads.iter_mut().find(|th| th.id == t)
        {
            th.name = n;
        }
        for id in &ids {
            if let Some(o) = d.object_mut(*id) {
                o.thread = t;
            }
        }
        d.remove_unused_threads();
        t
    })?;
    out(json!({"thread": t.0, "objects": ids.iter().map(|i| i.0).collect::<Vec<_>>()}))
}

fn move_objects(s: &mut Session, a: &Args) -> Result<Out, String> {
    let ids = objects_of(s, a)?;
    let (x, y, dx, dy) = (a.number("x")?, a.number("y")?, a.number("dx")?, a.number("dy")?);
    if (x.is_some() && dx.is_some()) || (y.is_some() && dy.is_some()) {
        return Err("give dx or x, and dy or y, not both".into());
    }
    let c = boxes(s.design(), &ids).map(|b| b.center()).unwrap_or(Point::ZERO);
    let t = Transform::translate(
        x.map_or(dx.unwrap_or(0.0), |x| x - c.x),
        y.map_or(dy.unwrap_or(0.0), |y| y - c.y),
    );
    transform(s, "Move", &ids, t)
}

fn scale_objects(s: &mut Session, a: &Args) -> Result<Out, String> {
    let ids = objects_of(s, a)?;
    let f = a.number("factor")?;
    let (sx, sy) = (
        a.number("sx")?.or(f).unwrap_or(1.0),
        a.number("sy")?.or(f).unwrap_or(1.0),
    );
    if !(sx > 0.0 && sy > 0.0) {
        return Err("the factors must be more than 0".into());
    }
    let t = about_middle(s.design(), &ids, Transform::scale(sx, sy))?;
    transform(s, "Scale", &ids, t)
}

fn rotate_objects(s: &mut Session, a: &Args) -> Result<Out, String> {
    let ids = objects_of(s, a)?;
    let t = about_middle(s.design(), &ids, Transform::rotate_deg(a.need_number("degrees")?))?;
    transform(s, "Rotate", &ids, t)
}

fn delete_objects(s: &mut Session, a: &Args) -> Result<Out, String> {
    let ids = objects_of(s, a)?;
    s.change("Delete", |d| {
        for id in &ids {
            d.remove_object(*id);
        }
        d.remove_unused_threads();
    })?;
    out(json!({"deleted": ids.iter().map(|i| i.0).collect::<Vec<_>>(), "objects": s.design().objects().len()}))
}

fn undo(s: &mut Session, _: &Args) -> Result<Out, String> {
    let label = s.doc.undo().ok_or("nothing to undo")?;
    out(json!({"undone": label, "objects": s.design().objects().len()}))
}

fn redo(s: &mut Session, _: &Args) -> Result<Out, String> {
    let label = s.doc.redo().ok_or("nothing to redo")?;
    out(json!({"redone": label, "objects": s.design().objects().len()}))
}

fn run_script(s: &mut Session, a: &Args) -> Result<Out, String> {
    let source = a.need_text("source")?;
    let selection = a.ids("ids")?;
    let mut inputs = BTreeMap::new();
    for (k, v) in a.object("inputs")?.into_iter().flatten() {
        let value = match v {
            Value::Number(n) => emb_script::InputValue::Number(n.as_f64().unwrap_or(0.0)),
            Value::Bool(b) => emb_script::InputValue::Bool(*b),
            Value::String(t) => emb_script::InputValue::Text(t.clone()),
            _ => return Err(format!("the input `{k}` must be a number, true or false, or text")),
        };
        inputs.insert(k.clone(), value);
    }
    let catalog = MachineCatalog::builtin();
    let profile = s.design().machine.as_deref().and_then(|m| catalog.get(m));
    let outcome = emb_script::run(
        source,
        s.design(),
        &selection,
        &inputs,
        profile,
        &emb_script::Limits::default(),
    )
    .map_err(|e| match e.line {
        Some(l) => format!("the script stopped at line {l}: {}", e.message),
        None => format!("the script stopped: {}", e.message),
    })?;
    if outcome.changed {
        let new = outcome.design;
        s.change("Run script", |d| *d = new)?;
    }
    out(json!({
        "changed": outcome.changed,
        "messages": outcome.messages,
        "selection": outcome.selection.iter().map(|i| i.0).collect::<Vec<_>>(),
    }))
}

fn check_design(s: &mut Session, _: &Args) -> Result<Out, String> {
    let d = s.design();
    let opts = plan_options(d, None).unwrap_or_default();
    let p = plan(d, &opts);
    let mut warnings: Vec<Value> = Vec::new();
    if let Some(h) = hoop_of(d) {
        for (object, at) in outside_hoop(d, &h, &p) {
            warnings.push(json!({
                "object": object.0,
                "text": format!("Stitches outside the field of the hoop {} ({} × {} mm)", h.name, h.width_mm, h.height_mm),
                "at_mm": [round(at.x), round(at.y)],
            }));
        }
    }
    warnings.extend(
        p.warnings
            .iter()
            .map(|w| json!({"object": w.object.map(|o| o.0), "text": w.message})),
    );
    // Travel that shows (P22 Milestone 1, ADR 0167): the limits of Design
    // info.
    if !emb_stitch::lace::is_lace(d, &emb_model::FabricCatalog::builtin()) {
        let shown = emb_stitch::shown_travel::shown_travel(&p);
        for o in shown.by_object() {
            if o.on_stitches_mm > 2.0 {
                warnings.push(json!({"object": o.object.0, "text": format!("Travel shows on top of sewn stitches: {:.1} mm", o.on_stitches_mm)}));
            }
            if let Some(piece) = o.longest_on_fabric.map(|i| &shown.pieces[i])
                && piece.length() >= 4.0
            {
                warnings.push(json!({"object": o.object.0, "text": format!("Travel shows on the fabric: {:.1} mm in one piece", piece.length())}));
            }
        }
    }
    if let Some(lace) = crate::lace_report(d, None)? {
        warnings.push(json!({"object": null, "text": lace}));
    }
    // The stitch objects as a machine file (ADR 0172).
    let stitch_ids: Vec<ObjectId> = d
        .objects()
        .iter()
        .filter(|o| matches!(o.kind, ObjectKind::Stitches { .. }))
        .map(|o| o.id)
        .collect();
    if !stitch_ids.is_empty() {
        let (records, owner) = emb_model::records_of_stitch_objects(d, &stitch_ids);
        let c = emb_art::file_check::check(&records, &opts.for_design(d));
        for f in &c.found {
            let object = owner.get(f.record).and_then(|&k| stitch_ids.get(k)).map(|i| i.0);
            warnings
                .push(json!({"object": object, "text": f.fault.cli_text(), "at_mm": [round(f.at.x), round(f.at.y)]}));
        }
    }
    out(json!({"warnings": warnings, "stitches": numbers(d)}))
}

fn picture(s: &mut Session, a: &Args) -> Result<Out, String> {
    // A picture is written only to a PNG file (not over a project).
    if let Some(path) = a.text("path")?
        && !path.to_lowercase().ends_with(".png")
    {
        return Err(format!("`path` must be a .png file (it is {path:?})"));
    }
    let d = s.design();
    let b = d.view_extents().ok_or("the design has nothing to show")?;
    let mut px = a.number("px_per_mm")?.unwrap_or(4.0).clamp(0.5, 20.0);
    let side = b.width().max(b.height()) + 4.0;
    if side * px > 2000.0 {
        px = 2000.0 / side;
    }
    let p = plan(d, &plan_options(d, None).unwrap_or_default());
    let preset = emb_model::FabricCatalog::builtin().of(d);
    let flat = a.flag("flat")?.unwrap_or(false);
    let plain = a.text("background")?.is_some();
    // A color that the agent gives is plain (no weave of the fabric); with
    // no color the fabric of the design shows (its weave in its color, else
    // white).
    let look = if plain && !flat {
        emb_render::raster::Look::three_d(None)
    } else {
        crate::picture_look(d, flat)
    };
    let (background, ground) = match a.text("background")? {
        Some(t) if t.eq_ignore_ascii_case("none") => (None, None),
        Some(t) => (Some(parse_color(t)?), None),
        None => (
            Some(d.background.unwrap_or(Rgb(255, 255, 255))),
            Some(emb_render::fabric::Ground::of(&preset, d)),
        ),
    };
    let spans = p.spans.iter().map(|s| (s.object, s.records.clone()));
    let pic = emb_render::picture::design_picture(d, &p.stitches, spans, ground, px, 2.0, background, &look);
    let png = pic.png();
    let mut value = json!({"width": pic.width, "height": pic.height, "px_per_mm": round(px)});
    if let Some(path) = a.text("path")? {
        std::fs::write(path, &png).map_err(|e| format!("{path}: {e}"))?;
        value["file"] = json!(path);
    }
    Ok(Out { value, png: Some(png) })
}

fn export(s: &mut Session, a: &Args) -> Result<Out, String> {
    let folder = PathBuf::from(a.need_text("folder")?);
    std::fs::create_dir_all(&folder).map_err(|e| format!("{}: {e}", folder.display()))?;
    let d = s.design();
    let name = file_name_of(&d.name);
    let proj = folder.join(format!("{name}.{}", project::FILE_EXTENSION));
    project::save(d, &proj).map_err(|e| format!("{}: {e}", proj.display()))?;
    let o = crate::OutputOptions {
        formats: a
            .texts("formats")?
            .iter()
            .map(|f| f.trim_start_matches('.').to_lowercase())
            .collect(),
        worksheet: a.flag("worksheet")?.unwrap_or(false),
        approval: a.flag("approval")?.unwrap_or(false),
        picture: a.flag("picture")?.unwrap_or(false),
        svg: false,
        paper: emb_render::page::Paper::default(),
        fabric: None,
        barcode: emb_render::barcode::BarcodeKind::None,
        stone_template: false,
        stone_clearance_mm: 0.3,
        flat_picture: false,
        picture_dpi: crate::DEFAULT_DPI,
        clear_picture: false,
        sewout_gif: false,
    };
    let (files, _) = crate::output(&proj, &folder, None, &o)?;
    let mut files: Vec<String> = files.iter().map(|f| f.display().to_string()).collect();
    if !files.contains(&proj.display().to_string()) {
        files.insert(0, proj.display().to_string());
    }
    s.doc.mark_saved();
    s.path = Some(proj);
    out(json!({"files": files}))
}

fn test_swatches(s: &mut Session, a: &Args) -> Result<Out, String> {
    let ids = objects_of(s, a)?;
    let name = a.need_text("setting")?;
    let setting = crate::swatch_setting_named(name)
        .ok_or_else(|| format!("no swatch setting {name:?} (list_catalog settings)"))?;
    let copies = a
        .number("copies")?
        .map_or(emb_letter::swatch::DEFAULT_COPIES, |c| c.max(0.0) as usize);
    let made = crate::swatch_design(s.design(), &ids, setting, a.number("from")?, a.number("to")?, copies)?;
    let path = PathBuf::from(a.need_text("path")?);
    project::save(&made.design, &path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut v = json!({
        "file": path.display().to_string(),
        "values": made.values.iter().map(|v| round(*v)).collect::<Vec<_>>(),
        "grid": [made.columns, made.rows],
        "size_mm": [round(made.size.0), round(made.size.1)],
        "fits_the_hoop": made.fits,
    });
    if !made.fits {
        v["warning"] = json!(
            "The swatches do not fit the field of the hoop: use fewer copies, a smaller object, or a larger hoop."
        );
    }
    out(v)
}

fn quicker_to_sew(s: &mut Session, a: &Args) -> Result<Out, String> {
    use emb_model::quicker::{QuickerToSew, apply};
    let standard = QuickerToSew::STANDARD;
    let q = QuickerToSew {
        merge_threads: Some(
            a.number("merge_threads")?
                .or(standard.merge_threads)
                .unwrap_or(0.0)
                .max(0.0),
        ),
        small_details_mm: a
            .number("small_details_mm")?
            .or(standard.small_details_mm)
            .filter(|v| *v > 0.0),
        fewer_color_changes: a.flag("fewer_color_changes")?.unwrap_or(standard.fewer_color_changes),
        gap_mm: Some(
            a.number("gap_mm")?
                .or(standard.gap_mm)
                .unwrap_or(3.0)
                .clamp(0.0, emb_model::quicker::MAX_TRIM_GAP_MM),
        ),
    };
    let before = numbers(s.design());
    // How much each thread sews: the thread that sews more stays.
    let d = s.design().clone();
    let p = plan(&d, &plan_options(&d, None).unwrap_or_default());
    let mut sews: std::collections::HashMap<emb_model::ThreadId, usize> = Default::default();
    for span in &p.spans {
        if let Some(o) = d.object(span.object) {
            *sews.entry(o.thread).or_default() += span.records.len();
        }
    }
    let sews_of = |t| sews.get(&t).copied().unwrap_or(0);
    let mut after_design = d.clone();
    let report = apply(&mut after_design, &q, &sews_of);
    let after = numbers(&after_design);
    let changes = !after_design.same_content(&d);
    let dry = a.flag("dry_run")?.unwrap_or(false);
    if !dry && changes {
        s.change("Quicker to Sew", |d| {
            apply(d, &q, &sews_of);
        })?;
    }
    out(json!({
        "before": before,
        "after": after,
        "merged_threads": report.merged.iter().map(|(gone, stays)| json!({"gone": gone.0, "now": stays.0})).collect::<Vec<_>>(),
        "removed_objects": report.removed.iter().map(|o| o.0).collect::<Vec<_>>(),
        "trim_distance_mm": {"before": d.trim_gap_mm, "after": after_design.trim_gap_mm},
        "changed": !dry && changes,
        "would_change": dry && changes,
    }))
}

fn check_machine_file(_: &mut Session, a: &Args) -> Result<Out, String> {
    let input = read_input(Path::new(a.need_text("path")?))?;
    let st = crate::file_stitches(&input)?;
    let opts = crate::file_check_options(&input, a.text("machine")?)?;
    let c = emb_art::file_check::check(&st, &opts);
    let counts: Map<String, Value> = emb_art::file_check::Fault::ALL
        .iter()
        .map(|f| (f.key().to_string(), json!(c.count(*f))))
        .collect();
    let needle = st
        .records
        .iter()
        .filter(|r| r.kind == emb_model::StitchKind::Stitch)
        .map(|r| r.pos);
    let mut v = json!({
        "faults": counts,
        "report": crate::check_report(&c),
        "box_mm": BBox::from_points(needle.clone()).map(box_value),
        "stitches": st.stitch_count(),
        "colors": st.color_count(),
        "trims": st.trim_count(),
        "threads": st.threads.iter().map(|t| json!({"color": t.color.to_hex(), "name": t.name})).collect::<Vec<_>>(),
    });
    if let Some(id) = a.text("hoop")? {
        let h = HoopCatalog::builtin()
            .get(id)
            .cloned()
            .ok_or_else(|| format!("no hoop {id:?} (list_catalog hoops)"))?;
        // A machine file has its middle at the middle of the hoop.
        v["fits_hoop"] = json!(needle.clone().all(|p| h.contains(p, 0.0)));
    }
    out(v)
}

fn fix_machine_file(_: &mut Session, a: &Args) -> Result<Out, String> {
    let input = read_input(Path::new(a.need_text("path")?))?;
    let st = crate::file_stitches(&input)?;
    let machine = a.text("machine")?;
    let opts = crate::file_check_options(&input, machine)?;
    let fixes = a.texts("fixes")?;
    let which = if fixes.is_empty() {
        emb_art::file_check::Fixes::ALL
    } else {
        crate::parse_fixes(&fixes.join(","))?
    };
    let (fixed, n) = emb_art::file_check::fix(&st, &opts, which);
    let out_path = a.need_text("out")?;
    let catalog = MachineCatalog::builtin();
    crate::write_output_for(&fixed, Path::new(out_path), machine.and_then(|m| catalog.get(m)))?;
    out(json!({
        "file": out_path,
        "joined": n.joined,
        "removed": n.removed,
        "split": n.split,
        "trimmed": n.trimmed,
        "locked": n.locked,
    }))
}

fn show(s: &mut Session, _: &Args) -> Result<Out, String> {
    let path = match (&s.path, &s.shown) {
        (Some(p), _) => {
            project::save(s.design(), p).map_err(|e| format!("{}: {e}", p.display()))?;
            s.doc.mark_saved();
            p.clone()
        }
        // A design with no file: a file of this session in the temp folder
        // (a clean name, so it stays in the folder). The design stays not
        // saved: save_design writes it where the agent says.
        (None, shown) => {
            let p = match shown {
                Some(p) => p.clone(),
                None => {
                    let dir = std::env::temp_dir().join("ShokujoDX agent");
                    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
                    dir.join(format!(
                        "{} {}.{}",
                        file_name_of(&s.design().name),
                        std::process::id(),
                        project::FILE_EXTENSION
                    ))
                }
            };
            project::save(s.design(), &p).map_err(|e| format!("{}: {e}", p.display()))?;
            s.shown = Some(p.clone());
            p
        }
    };
    let program = program()?;
    // The program gives the file to the program that runs (its inbox, P22
    // Milestone 8), else it opens a window. Its output is not this output.
    let mut child = std::process::Command::new(&program)
        .arg(std::path::absolute(&path).unwrap_or_else(|_| path.clone()))
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| format!("{}: {e}", program.display()))?;
    // Its end is read, so it leaves no process entry behind.
    std::thread::spawn(move || child.wait());
    out(json!({"shown": path.display().to_string()}))
}

/// A name for a file from the name of a design: letters, digits, spaces,
/// `-` and `_` (other characters become `_`); "design" for a name with
/// none of them.
fn file_name_of(name: &str) -> String {
    let clean: String = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || " -_".contains(c) {
                c
            } else {
                '_'
            }
        })
        .collect();
    if clean.trim().is_empty() {
        "design".to_string()
    } else {
        clean
    }
}

/// The program ShokujoDX: `SHOKUJODX_PROGRAM`, else next to `emb-cli`.
fn program() -> Result<PathBuf, String> {
    if let Some(p) = std::env::var_os("SHOKUJODX_PROGRAM") {
        return Ok(PathBuf::from(p));
    }
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let dir = exe.parent().ok_or("no folder of emb-cli")?;
    let names: &[&str] = if cfg!(windows) {
        &["ShokujoDX.exe", "emb-app.exe"]
    } else {
        &["ShokujoDX", "shokujodx", "emb-app"]
    };
    names
        .iter()
        .map(|n| dir.join(n))
        .find(|p| p.is_file())
        .ok_or_else(|| format!("ShokujoDX is not next to emb-cli ({})", dir.display()))
}

fn list_catalog(_: &mut Session, a: &Args) -> Result<Out, String> {
    let what = a.need_text("what")?.to_lowercase();
    let list: Vec<Value> = match what.as_str() {
        "fonts" => font_list().clone(),
        "machines" => MachineCatalog::builtin()
            .machines
            .iter()
            .map(|m| {
                json!({
                    "id": m.id,
                    "name": m.name,
                    "brand": m.brand,
                    "formats": m.formats,
                    "default_format": m.default_format,
                    "hoops": m.hoops,
                })
            })
            .collect(),
        "hoops" => HoopCatalog::builtin()
            .hoops
            .iter()
            .map(|h| json!({"id": h.id, "name": h.name, "width": h.width_mm, "height": h.height_mm}))
            .collect(),
        "fabrics" => emb_model::FabricCatalog::builtin()
            .fabrics
            .iter()
            .map(|f| json!({"id": f.id, "name": f.name, "description": f.description, "topping": f.topping}))
            .collect(),
        "settings" => emb_letter::swatch::SwatchSetting::ALL
            .iter()
            .map(|s| json!(s.name().to_lowercase()))
            .collect(),
        other => {
            return Err(format!(
                "`what` is fonts, machines, hoops, fabrics or settings, not {other:?}"
            ));
        }
    };
    let mut m = Map::new();
    m.insert(what, Value::Array(list));
    out(Value::Object(m))
}

// --- Helpers -------------------------------------------------------------

/// The bundled embroidery fonts: name, kind, size range and score (read
/// one time).
fn font_list() -> &'static Vec<Value> {
    static LIST: std::sync::OnceLock<Vec<Value>> = std::sync::OnceLock::new();
    LIST.get_or_init(|| {
        emb_letter::bundled::EMBROIDERY
            .iter()
            .map(|(name, _)| {
                let font = emb_letter::bundled::embroidery(name);
                let fill = emb_letter::bundled::is_fill(name);
                json!({
                    "name": name,
                    "kind": if fill { "fill letters" } else { "satin columns" },
                    "sizes_mm": font.map(|f| [round(f.min_height_mm), round(f.max_height_mm)]),
                    "score": emb_letter::bundled::quality(name).map(|q| q.score),
                })
            })
            .collect()
    })
}

/// The hoop of the design (none for an open field).
fn hoop_of(d: &Design) -> Option<emb_model::HoopPreset> {
    match &d.hoop.selection {
        HoopSelection::Preset { id } => HoopCatalog::builtin().get(id).cloned(),
        HoopSelection::OpenField => None,
    }
}

/// The objects with stitches outside the field of the hoop `h`, and the
/// first such stitch of each (design coordinates).
fn outside_hoop(d: &Design, h: &emb_model::HoopPreset, p: &emb_stitch::StitchPlan) -> Vec<(ObjectId, Point)> {
    let back = d.from_hoop_transform();
    p.spans
        .iter()
        .filter_map(|span| {
            p.stitches.records[span.records.clone()]
                .iter()
                .filter(|r| r.kind == emb_model::StitchKind::Stitch)
                .find(|r| !h.contains(r.pos, 0.0))
                .map(|r| (span.object, back.apply(r.pos)))
        })
        .collect()
}

/// The shape of the object is in the field of the hoop (`None`: no hoop).
/// The stitches can go out a little more (pull compensation):
/// check_design checks them.
fn shape_fits_hoop(d: &Design, id: ObjectId) -> Option<bool> {
    let h = hoop_of(d)?;
    let o = d.object(id)?;
    let t = d.output_transform();
    if matches!(o.kind, ObjectKind::CrossStitch(_) | ObjectKind::Bling(_)) {
        let b = o.kind.control_bbox()?;
        let corners = [b.min, b.max, Point::new(b.min.x, b.max.y), Point::new(b.max.x, b.min.y)];
        return Some(corners.iter().all(|p| h.contains(t.apply(*p), 0.0)));
    }
    Some(
        o.kind
            .paths()
            .flat_map(|p| p.flatten(0.05))
            .all(|l| l.points.iter().all(|p| h.contains(t.apply(*p), 0.0))),
    )
}

/// Puts `patch` in `into`: an object goes into the object that is there
/// (a group of settings keeps the settings that the patch does not name),
/// except when the patch changes its kind (`mode` or `type`); other values
/// take the place of the value that is there.
fn merge_value(into: &mut Value, patch: Value) {
    match (into, patch) {
        (Value::Object(old), Value::Object(new))
            if ["mode", "type"]
                .iter()
                .all(|tag| new.get(*tag).is_none_or(|t| old.get(*tag) == Some(t))) =>
        {
            for (k, v) in new {
                merge_value(old.entry(k).or_insert(Value::Null), v);
            }
        }
        (slot, v) => *slot = v,
    }
}

/// The box of the shapes of `ids` (the curves, not their control points).
fn boxes(d: &Design, ids: &[ObjectId]) -> Option<BBox> {
    ids.iter()
        .filter_map(|id| d.object(*id))
        .filter_map(|o| kind_box(&o.kind))
        .reduce(BBox::union)
}

fn box_value(b: BBox) -> Value {
    json!({"x": round(b.min.x), "y": round(b.min.y), "width": round(b.width()), "height": round(b.height())})
}

/// Moves, scales or rotates `ids` by `t` as one undo step.
fn transform(s: &mut Session, label: &str, ids: &[ObjectId], t: Transform) -> Result<Out, String> {
    // No undo step for a move of 0 (or a scale of 1).
    if t != Transform::IDENTITY {
        s.change(label, |d| d.transform_objects(ids, &t))?;
    }
    let b = boxes(s.design(), ids);
    out(json!({
        "objects": ids.iter().map(|i| i.0).collect::<Vec<_>>(),
        "box_mm": b.map(box_value),
    }))
}

/// `t` about the middle of the box of `ids`.
fn about_middle(d: &Design, ids: &[ObjectId], t: Transform) -> Result<Transform, String> {
    let c = boxes(d, ids).ok_or("the objects have no size")?.center();
    Ok(Transform::translate(-c.x, -c.y)
        .then(t)
        .then(Transform::translate(c.x, c.y)))
}

/// The thread of the color, or the first thread (a new one if the design
/// has none).
fn thread_of(d: &mut Design, color: Option<Rgb>) -> emb_model::ThreadId {
    match (color, d.threads.first()) {
        (Some(c), _) => d.thread_for_color(c),
        (None, Some(t)) => t.id,
        (None, None) => d.thread_for_color(Rgb(0x1f, 0x4e, 0x9e)),
    }
}

/// The stitch numbers of a design.
fn numbers(d: &Design) -> Value {
    let p = plan(d, &plan_options(d, None).unwrap_or_default());
    let shown = emb_stitch::shown_travel::shown_travel(&p);
    json!({
        "stitches": p.stitches.stitch_count(),
        "colors": p.stitches.color_count(),
        "trims": p.stitches.trim_count(),
        "travel_on_stitches_mm": round(shown.on_stitches_mm()),
        "warnings": p.warnings.len(),
    })
}

/// One object for a reply.
fn object_value(d: &Design, id: ObjectId) -> Value {
    let Some(o) = d.object(id) else { return Value::Null };
    let b = boxes(d, &[id]);
    json!({
        "id": id.0,
        "kind": o.type_name(),
        "thread": o.thread.0,
        "box_mm": b.map(box_value),
        "fits_hoop": shape_fits_hoop(d, id),
        "settings": shown_settings(&o.kind),
    })
}

/// The settings of lettering that the layout makes (not for the agent).
const LAID_OUT: [&str; 5] = ["letters", "edits", "placement", "envelope_box", "range_mm"];

/// All the settings of `kind` (of lettering also what the layout makes).
fn all_settings(kind: &ObjectKind) -> Option<Value> {
    let v = match kind {
        ObjectKind::Running { params, .. } => serde_json::to_value(params),
        ObjectKind::Satin { params, .. } => serde_json::to_value(params),
        ObjectKind::Fill { params, .. } => serde_json::to_value(params),
        ObjectKind::Applique { params, .. } => serde_json::to_value(params),
        ObjectKind::Vector { style, .. } => serde_json::to_value(style),
        ObjectKind::Lettering(l) => serde_json::to_value(l),
        _ => return None,
    };
    v.ok()
}

/// The settings that the agent sees and changes.
fn shown_settings(kind: &ObjectKind) -> Value {
    let Some(mut v) = all_settings(kind) else {
        return Value::Null;
    };
    if let (ObjectKind::Lettering(_), Some(m)) = (kind, v.as_object_mut()) {
        for k in LAID_OUT {
            m.remove(k);
        }
    }
    v
}

/// Puts the settings `v` (all of them) in `kind`.
fn put_settings(kind: &mut ObjectKind, v: Value) -> Result<(), String> {
    let e = |e: serde_json::Error| e.to_string();
    match kind {
        ObjectKind::Running { params, .. } => *params = serde_json::from_value(v).map_err(e)?,
        ObjectKind::Satin { params, .. } => *params = serde_json::from_value(v).map_err(e)?,
        ObjectKind::Fill { params, .. } => *params = serde_json::from_value(v).map_err(e)?,
        ObjectKind::Applique { params, .. } => *params = serde_json::from_value(v).map_err(e)?,
        ObjectKind::Vector { style, .. } => *style = serde_json::from_value(v).map_err(e)?,
        ObjectKind::Lettering(l) => **l = serde_json::from_value(v).map_err(e)?,
        _ => return Err("this object has no settings that the tools change".into()),
    }
    Ok(())
}

/// `key` is a setting of `kind`: it is in its settings, or a value makes
/// it so (a setting at its default value is not written).
fn is_setting(kind: &ObjectKind, key: &str) -> bool {
    let Some(Value::Object(all)) = all_settings(kind) else {
        return false;
    };
    if all.contains_key(key) {
        return true;
    }
    [
        json!(1.0),
        json!(1),
        json!(true),
        json!("x"),
        json!({}),
        json!([1.0, 1.0]),
    ]
    .into_iter()
    .any(|probe| {
        let mut m = all.clone();
        m.insert(key.into(), probe);
        let mut k = kind.clone();
        put_settings(&mut k, Value::Object(m)).is_ok() && all_settings(&k).is_some_and(|v| v.get(key).is_some())
    })
}

/// `kind` with the settings of `patch`; lettering is laid out again.
fn with_settings(kind: &ObjectKind, patch: &Map<String, Value>) -> Result<ObjectKind, String> {
    let Some(Value::Object(mut all)) = all_settings(kind) else {
        return Err(format!("a {} has no settings that the tools change", kind.type_name()));
    };
    let lettering = matches!(kind, ObjectKind::Lettering(_));
    // The font of lettering by its name, as add_lettering takes it.
    let mut patch = patch.clone();
    if lettering && let Some(Value::String(name)) = patch.get("font").cloned() {
        let font = emb_letter::bundled::embroidery(&name)
            .ok_or_else(|| format!("no bundled embroidery font {name:?} (list_catalog fonts)"))?;
        let font = serde_json::to_value(emb_letter::LetterSource::font_ref(&font)).map_err(|e| e.to_string())?;
        patch.insert("font".into(), font);
    }
    let unknown: Vec<&str> = patch
        .keys()
        .map(String::as_str)
        .filter(|k| (lettering && LAID_OUT.contains(k)) || !is_setting(kind, k))
        .collect();
    if !unknown.is_empty() {
        let shown = shown_settings(kind);
        let names: Vec<&str> = shown
            .as_object()
            .map(|m| m.keys().map(String::as_str).collect())
            .unwrap_or_default();
        return Err(format!(
            "`{}` is not a setting of a {}. Its settings: {}.",
            unknown.join("`, `"),
            kind.type_name(),
            names.join(", ")
        ));
    }
    for (k, v) in patch {
        merge_value(all.entry(k).or_insert(Value::Null), v);
    }
    let mut new = kind.clone();
    put_settings(&mut new, Value::Object(all)).map_err(|e| format!("these settings do not fit: {e}"))?;
    if let ObjectKind::Lettering(l) = &mut new {
        if l.text.chars().count() > MAX_TEXT_CHARS {
            return Err(format!("`text` has more than {MAX_TEXT_CHARS} characters"));
        }
        let font = emb_letter::bundled::embroidery(&l.font.family).ok_or_else(|| {
            format!(
                "the font {:?} is not a bundled embroidery font: the tools lay out only those",
                l.font.family
            )
        })?;
        emb_letter::relayout(l, &font);
    }
    Ok(new)
}

/// The kinds of the SVG path text `d` (in mm): vector shapes. The SVG
/// reader of the program reads the path (all the commands of SVG); a
/// viewBox of mm keeps the coordinates, and they are rounded to 0.1 µm (the
/// reader keeps them in 32-bit numbers).
fn svg_kinds(d: &str) -> Result<Vec<ObjectKind>, String> {
    if d.contains(['"', '<', '>', '&']) {
        return Err("the path text has characters that SVG path data does not have".into());
    }
    let svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="1000mm" height="1000mm" viewBox="0 0 1000 1000"><path d="{d}" fill="none" stroke="black"/></svg>"#
    );
    let import =
        emb_formats::artwork::import_artwork("svg", svg.as_bytes(), true).map_err(|e| format!("the path text: {e}"))?;
    let mut kinds: Vec<ObjectKind> = import.objects.into_iter().map(|o| o.kind).collect();
    if kinds.is_empty() {
        return Err("the path text gives no shape".into());
    }
    for k in &mut kinds {
        for p in k.paths_mut() {
            *p = rounded(p);
        }
    }
    Ok(kinds)
}

/// The path of the SVG path text `d`, in mm.
fn svg_outline(d: &str) -> Result<Outline, String> {
    let kinds = svg_kinds(d)?;
    Ok(Outline {
        cmds: kinds
            .iter()
            .flat_map(|k| k.paths().flat_map(|p| p.cmds.clone()))
            .collect(),
    })
}

fn rounded(p: &Outline) -> Outline {
    let r = |q: Point| Point::new((q.x * 1e4).round() / 1e4, (q.y * 1e4).round() / 1e4);
    Outline {
        cmds: p
            .cmds
            .iter()
            .map(|c| match *c {
                PathCmd::MoveTo(a) => PathCmd::MoveTo(r(a)),
                PathCmd::LineTo(a) => PathCmd::LineTo(r(a)),
                PathCmd::CubicTo(a, b, e) => PathCmd::CubicTo(r(a), r(b), r(e)),
                PathCmd::Close => PathCmd::Close,
            })
            .collect(),
    }
}

/// The parts of a path (each starts at a move), each closed.
fn subpaths(p: &Outline) -> Vec<Outline> {
    let mut out: Vec<Outline> = Vec::new();
    for c in &p.cmds {
        match c {
            PathCmd::MoveTo(_) => out.push(Outline { cmds: vec![*c] }),
            _ => match out.last_mut() {
                Some(o) => o.cmds.push(*c),
                None => out.push(Outline { cmds: vec![*c] }),
            },
        }
    }
    for o in &mut out {
        if !matches!(o.cmds.last(), Some(PathCmd::Close)) {
            o.cmds.push(PathCmd::Close);
        }
    }
    out.retain(|o| o.cmds.len() > 2);
    out
}

/// The box of the shapes of `k`: the curves, not their control points.
fn kind_box(k: &ObjectKind) -> Option<BBox> {
    if matches!(k, ObjectKind::CrossStitch(_) | ObjectKind::Bling(_)) {
        return k.control_bbox();
    }
    k.paths()
        .flat_map(|p| p.flatten(0.01))
        .filter_map(|l| BBox::from_points(l.points))
        .reduce(BBox::union)
        .or_else(|| k.control_bbox())
}

fn parse_color(s: &str) -> Result<Rgb, String> {
    let h = s.trim();
    Rgb::from_hex(&format!("#{}", h.strip_prefix('#').unwrap_or(h)))
        .ok_or_else(|| format!("{s:?} is not a color: use #rrggbb"))
}

fn round(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

/// `bytes` in base64 (for the pictures of the replies).
fn base64(bytes: &[u8]) -> String {
    const ABC: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let n = (u32::from(chunk[0]) << 16)
            | (u32::from(*chunk.get(1).unwrap_or(&0)) << 8)
            | u32::from(*chunk.get(2).unwrap_or(&0));
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(ABC[(n >> (18 - 6 * i)) as usize & 63] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_is_the_standard_one() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
    }

    /// The path text keeps its coordinates, also below 0.
    #[test]
    fn svg_path_text_keeps_its_millimetres() {
        let o = svg_outline("M -5 -3 L 25 -3 L 25 7 Z").unwrap();
        let b = o.control_bbox().unwrap();
        assert_eq!((b.min.x, b.min.y, b.max.x, b.max.y), (-5.0, -3.0, 25.0, 7.0));
        let circle = svg_outline("M -30 0 A 30 30 0 1 0 30 0 A 30 30 0 1 0 -30 0 Z").unwrap();
        let b = circle.control_bbox().unwrap();
        assert!((b.width() - 60.0).abs() < 0.01 && b.center().x.abs() < 0.01, "{b:?}");
        assert!(svg_outline("M 0 0 L 10 0\" onload=\"x").is_err());
    }

    /// A group of settings keeps the settings that a change does not name;
    /// a change of its kind takes its place.
    #[test]
    fn settings_go_into_their_groups() {
        let mut v = json!({"satin": {"spacing_mm": 0.4, "split_mm": 7.0, "underlay": {"mode": "auto"}}});
        merge_value(
            &mut v,
            json!({"satin": {"spacing_mm": 0.5, "underlay": {"mode": "none"}}}),
        );
        assert_eq!(
            v,
            json!({"satin": {"spacing_mm": 0.5, "split_mm": 7.0, "underlay": {"mode": "none"}}})
        );
        let mut v = json!({"underlay": {"mode": "custom", "layers": [{"type": "center_walk"}]}});
        merge_value(&mut v, json!({"underlay": {"mode": "light"}}));
        assert_eq!(v, json!({"underlay": {"mode": "light"}}));
    }

    /// Lines that are not messages get an error, and the session goes on:
    /// bytes that are not UTF-8, JSON that is not an object, a byte order
    /// mark (the audit of 2026-11-26: a line that was not UTF-8 stopped the
    /// server, and its design was lost).
    #[test]
    fn lines_that_are_not_messages_do_not_stop_the_server() {
        let mut input = Vec::new();
        input.extend_from_slice(b"\xff\xfe\n5\nnull\n");
        input.extend_from_slice("\u{feff}{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"ping\"}\n".as_bytes());
        input.extend_from_slice(b"{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"ping\"}\n");
        let mut out = Vec::new();
        serve(std::io::Cursor::new(input), &mut out).unwrap();
        let replies: Vec<Value> = String::from_utf8(out)
            .unwrap()
            .lines()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect();
        let codes: Vec<Value> = replies.iter().map(|r| r["error"]["code"].clone()).collect();
        assert_eq!(
            codes,
            [json!(-32700), json!(-32600), json!(-32600), Value::Null, Value::Null]
        );
        assert_eq!(
            (replies[3]["id"].clone(), replies[4]["id"].clone()),
            (json!(1), json!(2))
        );
    }

    /// A text of lettering longer than `MAX_TEXT_CHARS` is refused, as a new
    /// object and as a setting.
    #[test]
    fn a_very_long_text_of_lettering_is_refused() {
        let mut s = Session::new();
        let mut call = |name: &str, args: Value| {
            let m = json!({"jsonrpc": "2.0", "id": 1, "method": "tools/call",
                "params": {"name": name, "arguments": args}});
            s.handle(&m).unwrap()["result"].clone()
        };
        let long = "A".repeat(MAX_TEXT_CHARS + 1);
        assert_eq!(call("add_lettering", json!({"text": long}))["isError"], true);
        let r = call("add_lettering", json!({"text": "AB"}));
        assert_eq!(r["isError"], false, "{r}");
        let id = r["structuredContent"]["added"].clone();
        let r = call("set_settings", json!({"ids": [id], "settings": {"text": long}}));
        assert_eq!(r["isError"], true, "{r}");
    }

    /// Wrong calls that the audit of 2026-11-26 found: a color with a
    /// letter that is not ASCII (a panic), an id two times (a move of two
    /// times the distance), a move of 0 (an undo step), a picture over a
    /// file that is not a PNG file, a request with no method (no reply).
    #[test]
    fn wrong_calls_found_by_the_audit() {
        let mut s = Session::new();
        let mut call = |name: &str, args: Value| {
            let m = json!({"jsonrpc": "2.0", "id": 1, "method": "tools/call",
                "params": {"name": name, "arguments": args}});
            s.handle(&m).unwrap()["result"].clone()
        };
        for color in ["€abc", "#ab€c", "#12345", "ff00zz"] {
            let r = call(
                "add_shape",
                json!({"svg_path": "M 0 0 L 10 0", "stitch": "run", "color": color}),
            );
            assert_eq!(r["isError"], true, "{color}: {r}");
        }
        let r = call(
            "add_shape",
            json!({"svg_path": "M 0 0 L 10 0", "stitch": "run", "color": "FF0000"}),
        );
        assert_eq!(r["isError"], false, "{r}");
        let id = r["structuredContent"]["added"].clone();
        let x = |r: &Value| r["structuredContent"]["box_mm"]["x"].as_f64().unwrap();
        let moved = call("move_objects", json!({"ids": [id, id], "dx": 10}));
        assert_eq!(x(&moved), 10.0, "{moved}");
        let r = call("move_objects", json!({"ids": [id]}));
        assert_eq!(r["isError"], false, "{r}");
        assert_eq!(call("undo", json!({}))["structuredContent"]["undone"], "Move");
        let r = call("undo", json!({}));
        assert_eq!(r["structuredContent"]["undone"], "Add shape", "{r}");
        let r = call("picture", json!({"path": "design.embproj"}));
        assert_eq!(r["isError"], true, "{r}");
        assert!(!std::path::Path::new("design.embproj").exists());
        for m in [
            json!({"jsonrpc": "2.0", "id": 4}),
            json!({"jsonrpc": "2.0", "id": 5, "method": 7}),
        ] {
            let r = s.handle(&m).unwrap();
            assert_eq!(r["error"]["code"], -32600, "{r}");
        }
        // A reply of the client gets no reply.
        assert!(s.handle(&json!({"jsonrpc": "2.0", "id": 6, "result": {}})).is_none());
        assert_eq!(file_name_of("a/b\\..\\c"), "a_b____c");
        assert_eq!(file_name_of("../"), "___");
        assert_eq!(file_name_of(" "), "design");
    }
}
