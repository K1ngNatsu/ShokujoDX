//! The exit test of P22 Milestone 9 (ADR 0176): a session with `emb-cli
//! mcp` through pipes, as an MCP client has it. The handshake, the list of
//! tools and a call of each tool; it makes a badge (a round fill,
//! lettering, a satin border), checks it, and writes a picture and a PES
//! file that reads back. An argument that a tool does not take is refused,
//! and each change is one undo step.

use std::collections::BTreeSet;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use serde_json::{Value, json};

struct Client {
    child: Child,
    input: ChildStdin,
    output: BufReader<ChildStdout>,
    next: u64,
    called: BTreeSet<String>,
}

impl Client {
    fn start() -> Client {
        let mut child = Command::new(env!("CARGO_BIN_EXE_emb-cli"))
            .arg("mcp")
            // `show` starts this program in place of ShokujoDX: it stops at
            // once (an unknown command), and no window opens in the test.
            .env("SHOKUJODX_PROGRAM", env!("CARGO_BIN_EXE_emb-cli"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("emb-cli mcp starts");
        let input = child.stdin.take().unwrap();
        let output = BufReader::new(child.stdout.take().unwrap());
        Client {
            child,
            input,
            output,
            next: 1,
            called: BTreeSet::new(),
        }
    }

    fn send(&mut self, message: &Value) {
        let line = serde_json::to_string(message).unwrap();
        assert!(!line.contains('\n'));
        writeln!(self.input, "{line}").unwrap();
        self.input.flush().unwrap();
    }

    /// A request, and its reply (one line).
    fn request(&mut self, method: &str, params: Value) -> Value {
        let id = self.next;
        self.next += 1;
        self.send(&json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}));
        let mut line = String::new();
        self.output.read_line(&mut line).unwrap();
        let reply: Value = serde_json::from_str(&line).unwrap_or_else(|e| panic!("{e}: {line}"));
        assert_eq!(reply["jsonrpc"], "2.0");
        assert_eq!(reply["id"], id, "{reply}");
        reply
    }

    fn call(&mut self, name: &str, args: Value) -> Value {
        self.called.insert(name.to_string());
        let reply = self.request("tools/call", json!({"name": name, "arguments": args}));
        reply["result"].clone()
    }

    /// A call that works: its value.
    fn ok(&mut self, name: &str, args: Value) -> Value {
        let r = self.call(name, args);
        assert_eq!(r["isError"], false, "{name}: {r}");
        r["structuredContent"].clone()
    }

    /// A call that the tool refuses: its text.
    fn refused(&mut self, name: &str, args: Value) -> String {
        let r = self.call(name, args);
        assert_eq!(r["isError"], true, "{name}: {r}");
        r["content"][0]["text"].as_str().unwrap().to_string()
    }

    fn undo_steps(&mut self) -> u64 {
        self.ok("design_info", json!({}))["undo_steps"].as_u64().unwrap()
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        self.child.kill().ok();
    }
}

fn folder() -> PathBuf {
    let d = std::env::temp_dir().join(format!("emb-mcp-{}", std::process::id()));
    std::fs::remove_dir_all(&d).ok();
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn s(p: &Path) -> String {
    p.display().to_string()
}

#[test]
fn an_agent_makes_a_badge_through_the_mcp_server() {
    let dir = folder();
    let mut c = Client::start();

    // The handshake.
    let init = c.request(
        "initialize",
        json!({"protocolVersion": "2025-11-25", "capabilities": {}, "clientInfo": {"name": "test", "version": "1"}}),
    );
    assert_eq!(init["result"]["protocolVersion"], "2025-11-25");
    assert_eq!(init["result"]["serverInfo"]["name"], "shokujodx");
    assert!(init["result"]["capabilities"]["tools"].is_object());
    c.send(&json!({"jsonrpc": "2.0", "method": "notifications/initialized"}));
    // An older client gets its version; a newer one gets the newest.
    let old = c.request(
        "initialize",
        json!({"protocolVersion": "2024-11-05", "capabilities": {}}),
    );
    assert_eq!(old["result"]["protocolVersion"], "2024-11-05");
    assert_eq!(c.request("ping", json!({}))["result"], json!({}));
    let discover = c.request("server/discover", json!({}));
    assert!(
        discover["result"]["supportedVersions"]
            .as_array()
            .unwrap()
            .contains(&json!("2026-07-28"))
    );
    assert_eq!(c.request("no/such/method", json!({}))["error"]["code"], -32601);
    assert_eq!(
        c.request("tools/call", json!({"name": "no_such_tool"}))["error"]["code"],
        -32602
    );

    // The tools: each with a schema that takes no other argument, and the
    // tools that only read say so.
    let list = c.request("tools/list", json!({}));
    let tools = list["result"]["tools"].as_array().unwrap().clone();
    let names: BTreeSet<String> = tools.iter().map(|t| t["name"].as_str().unwrap().to_string()).collect();
    assert_eq!(names.len(), 25);
    for t in &tools {
        assert_eq!(t["inputSchema"]["type"], "object", "{t}");
        assert_eq!(t["inputSchema"]["additionalProperties"], false, "{t}");
        let reads = t["annotations"]["readOnlyHint"].as_bool().unwrap();
        let name = t["name"].as_str().unwrap();
        let reading = [
            "design_info",
            "list_objects",
            "check_design",
            "picture",
            "check_machine_file",
            "list_catalog",
        ];
        assert_eq!(reads, reading.contains(&name), "{name}");
        assert_eq!(t["description"].as_str().unwrap().ends_with("It only reads."), reads);
    }

    // The names to use.
    let fonts = c.ok("list_catalog", json!({"what": "fonts"}));
    assert!(fonts["fonts"].as_array().unwrap().contains(&json!("Block Sans")));
    let hoops = c.ok("list_catalog", json!({"what": "hoops"}));
    let hoop = hoops["hoops"]
        .as_array()
        .unwrap()
        .iter()
        .find(|h| h["width"].as_f64().unwrap() >= 100.0 && h["height"].as_f64().unwrap() >= 100.0)
        .unwrap()["id"]
        .clone();
    c.ok("list_catalog", json!({"what": "machines"}));

    // The badge: one undo step for each change.
    c.ok(
        "new_design",
        json!({"name": "Badge", "machine": "husqvarna_topaz20", "hoop": hoop}),
    );
    assert_eq!(c.undo_steps(), 0);
    let circle = |r: f64| format!("M {} 0 A {r} {r} 0 1 0 {r} 0 A {r} {r} 0 1 0 {} 0 Z", -r, -r);
    let fill = c.ok(
        "add_shape",
        json!({"svg_path": circle(30.0), "stitch": "fill", "color": "#d8e4f0"}),
    )["added"]
        .as_u64()
        .unwrap();
    let text = c.ok(
        "add_lettering",
        json!({"text": "STAR", "height_mm": 12, "x": 0, "y": 0, "color": "#c01818"}),
    )["added"]
        .as_u64()
        .unwrap();
    let border = c.ok(
        "add_shape",
        json!({"svg_path": circle(31.5), "stitch": "satin", "width_mm": 4, "color": "#1f4e9e"}),
    )["added"]
        .as_u64()
        .unwrap();
    assert_eq!(c.undo_steps(), 3);
    let objects = c.ok("list_objects", json!({}))["objects"].clone();
    let kinds: Vec<&str> = objects
        .as_array()
        .unwrap()
        .iter()
        .map(|o| o["kind"].as_str().unwrap())
        .collect();
    assert_eq!(kinds, ["Fill", "Lettering", "Satin"]);
    let b = &objects[0]["box_mm"];
    assert!((b["width"].as_f64().unwrap() - 60.0).abs() < 0.1, "{b}");
    assert!((b["x"].as_f64().unwrap() + 30.0).abs() < 0.1, "{b}");

    // An argument that a tool does not take is refused, with the names.
    let e = c.refused(
        "add_shape",
        json!({"svg_path": circle(5.0), "stitch": "run", "colour": "#000000"}),
    );
    assert!(
        e.contains("takes no argument `colour`") && e.contains("It takes: svg_path, stitch"),
        "{e}"
    );
    let e = c.refused("set_settings", json!({"ids": [fill], "settings": {"nonsense": 1}}));
    assert!(
        e.contains("`nonsense` is not a setting") && e.contains("spacing_mm"),
        "{e}"
    );
    assert_eq!(c.undo_steps(), 3, "a refused call changes nothing");

    // Changes.
    c.ok(
        "set_settings",
        json!({"ids": [fill], "settings": {"spacing_mm": 0.45, "angle_deg": 30}}),
    );
    let settings = &c.ok("list_objects", json!({"ids": [fill]}))["objects"][0]["settings"];
    assert_eq!(settings["spacing_mm"], 0.45);
    c.ok(
        "set_settings",
        json!({"ids": [text], "settings": {"text": "STARS", "height_mm": 10}}),
    );
    c.ok("move_objects", json!({"ids": [text], "dy": -2}));
    c.ok("rotate_objects", json!({"ids": [text], "degrees": 5}));
    c.ok("scale_objects", json!({"ids": [text], "factor": 1.1}));
    c.ok(
        "set_thread",
        json!({"ids": [border], "color": "#204080", "name": "Navy"}),
    );
    assert_eq!(c.undo_steps(), 9);
    // A change out of the range of a project is refused, with the reason.
    let e = c.refused("move_objects", json!({"ids": [fill], "dx": 2_000_000}));
    assert!(e.contains("refused"), "{e}");
    assert_eq!(c.undo_steps(), 9);
    assert_eq!(c.ok("undo", json!({}))["undone"], "Change thread");
    assert_eq!(c.ok("redo", json!({}))["redone"], "Change thread");

    // The checks, the picture, the machine file.
    let check = c.ok("check_design", json!({}));
    assert!(check["warnings"].is_array());
    assert!(check["stitches"]["stitches"].as_u64().unwrap() > 1000, "{check}");
    let png = dir.join("badge.png");
    let pic = c.call("picture", json!({"path": s(&png)}));
    assert_eq!(pic["content"][1]["type"], "image");
    assert_eq!(pic["content"][1]["mimeType"], "image/png");
    assert!(std::fs::read(&png).unwrap().starts_with(b"\x89PNG"));
    let out = dir.join("out");
    let files = c.ok(
        "export",
        json!({"folder": s(&out), "formats": ["pes"], "worksheet": true}),
    )["files"]
        .clone();
    let pes = files
        .as_array()
        .unwrap()
        .iter()
        .map(|f| PathBuf::from(f.as_str().unwrap()))
        .find(|f| f.extension().is_some_and(|e| e == "pes"))
        .expect("a PES file");
    let back = emb_formats::read_machine("pes", &std::fs::read(&pes).unwrap()).expect("the PES file reads back");
    assert!(back.stitch_count() > 1000);
    assert_eq!(back.color_count(), 3);

    // Save, swatches, Quicker to Sew, a script.
    let project = dir.join("badge.embproj");
    c.ok("save_design", json!({"path": s(&project)}));
    assert!(project.is_file());
    assert_eq!(c.ok("design_info", json!({}))["saved"], true);
    let sw = c.ok(
        "test_swatches",
        json!({"ids": [fill], "setting": "spacing", "from": 0.3, "to": 0.5, "copies": 3, "path": s(&dir.join("swatches.embproj"))}),
    );
    assert_eq!(sw["values"], json!([0.3, 0.4, 0.5]));
    let steps = c.undo_steps();
    let q = c.ok("quicker_to_sew", json!({"dry_run": true}));
    assert!(q["before"]["stitches"].is_u64() && q["after"]["stitches"].is_u64());
    assert_eq!(c.undo_steps(), steps, "a dry run changes nothing");
    let script = c.ok("run_script", json!({"source": "set_design_name(\"Badge 2\");"}));
    assert_eq!(script["changed"], true);
    assert_eq!(c.undo_steps(), steps + 1);

    // A machine file: check it, fix it.
    let checked = c.ok("check_machine_file", json!({"path": s(&pes)}));
    assert!(checked["faults"].is_object());
    let fixed = dir.join("fixed.dst");
    c.ok("fix_machine_file", json!({"path": s(&pes), "out": s(&fixed)}));
    assert!(fixed.is_file());

    // Delete and undo; show; a new design does not take unsaved work.
    c.ok("delete_objects", json!({"ids": [border]}));
    assert_eq!(c.ok("design_info", json!({}))["objects"], 2);
    c.ok("undo", json!({}));
    assert_eq!(c.ok("design_info", json!({}))["objects"], 3);
    let shown = c.ok("show", json!({}));
    assert_eq!(shown["shown"], s(&project));
    c.ok("move_objects", json!({"ids": [fill], "dx": 1}));
    let e = c.refused("new_design", json!({}));
    assert!(e.contains("not saved"), "{e}");
    c.ok("open_design", json!({"path": s(&project), "discard_changes": true}));
    assert_eq!(c.ok("design_info", json!({}))["objects"], 3);

    // Each tool was called.
    assert_eq!(c.called, names, "each tool once at least");
    drop(c);
    std::fs::remove_dir_all(&dir).ok();
}
