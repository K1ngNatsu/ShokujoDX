# ADR 0176: Agents through MCP

Date: 2026-11-25 · Status: Accepted (P22 Milestone 9,
`docs/design/p22-plan.md`).

## Context

The AI agent of the user (an assistant program that can use tools) can
make designs with the program if the program gives it tools. The Model
Context Protocol (MCP) is the common way to give tools to an agent. The
idea is from zigzag (an open-source program under Apache-2.0); the code is
our own. The program has no network code (ADR 0109), and the assistant of
P10 Milestone 9 stays benched: this is a way for the user's own agent to
use the program, with no AI in the program.

## Decision

- **`emb-cli mcp`** (`emb_cli::mcp::serve`) is an MCP server on standard
  input and output (the stdio transport): JSON-RPC 2.0, one message on
  each line, and nothing else on standard output.
  - The handshake `initialize` of MCP 2025-11-25. A client that asks for
    2025-06-18, 2025-03-26 or 2024-11-05 gets that version; another
    version gets 2025-11-25.
  - `server/discover` of MCP 2026-07-28 (no handshake), `ping`,
    `tools/list` and `tools/call`. The results have `resultType`
    `complete` (2026-07-28 needs it; the versions before it do not read
    it).
  - A message that is not JSON is error -32700, a batch -32600, an
    unknown method -32601, an unknown tool -32602. An error of a tool is
    a result with `isError` true and the reason, so that the agent can
    correct its call.
  - A result has the value of the tool as JSON text and as
    `structuredContent`; the picture is also an image (PNG in base64).
- **One design.** The server keeps one design (`emb_model::Document` with
  its history). Each change is one `Document::edit`: one undo step. A
  change that the document refuses (ADR 0139) gives its reason
  (`Document::take_refused`). `new_design` and `open_design` do not take
  the place of changes that are not saved unless `discard_changes` is
  true.
- **25 tools:** `new_design`, `open_design`, `save_design`,
  `design_info`, `list_objects`, `add_shape`, `add_lettering`,
  `set_settings`, `set_thread`, `move_objects`, `scale_objects`,
  `rotate_objects`, `delete_objects`, `undo`, `redo`, `run_script`,
  `check_design`, `picture`, `export`, `test_swatches`, `quicker_to_sew`
  (with `dry_run`), `check_machine_file`, `fix_machine_file`, `show` and
  `list_catalog`. Measures are mm from the middle of the hoop, x to the
  right and y down; angles are degrees.
- **Checked arguments.** The schema of each tool has
  `additionalProperties: false`. A tool refuses an argument that it does
  not take and names the ones that it takes, an argument that it needs
  and does not get, and a value of the wrong type (numbers must be real
  numbers). A tool that only reads has `readOnlyHint`, and its
  description ends with "It only reads."
- **Shapes** come from SVG path text in mm: the SVG reader of the program
  reads it (all the commands of SVG, also arcs) with a viewBox of mm, and
  the coordinates are rounded to 0.1 µm (the reader keeps 32-bit
  numbers). A fill takes the first closed part as its outline and the
  others as holes; a satin column takes the path as its middle line.
- **Settings.** `set_settings` changes the settings of objects by the
  names that `list_objects` shows (the saved form of the settings). A
  name that is not a setting is refused (a setting at its default value
  is not written, so the server tries values to find it). Lettering is
  laid out again with its bundled embroidery font; the server has only
  the bundled embroidery fonts.
- **Show.** `show` saves the design (to its file, else to the folder
  `ShokujoDX agent` of the temporary files) and starts the program next
  to `emb-cli` with the file (`SHOKUJODX_PROGRAM` names another program,
  for the tests). That start gives the file to the program that runs
  (the inbox of ADR 0175) or opens a window. The output of the program
  does not go to the output of the server.
- **A file that another program saves.** The program knows the time of
  each file that it opened or saved (`file_watch.rs`, `file_known`). Each
  second it compares the file of the tab in front (`poll_file_changes`).
  A file that changed gets the question File Changed: Open Again puts the
  design of the file in the tab with a new history and a new id
  (`AppState::replace_tab_design`), and Keep This Version keeps the tab.
- **Packages.** `emb-cli` is in each package next to the program (Windows,
  the macOS bundle, `usr/bin` on Linux), so the setup line of the help is
  the folder of the program and `emb-cli mcp`.
- The help topic Agents (four languages) has the setup and what the agent
  can do; the README names it.

## Consequences

- The exit test (`crates/emb-cli/tests/mcp.rs`) runs a session through
  pipes: the handshake, the list of tools and a call of each tool; it
  makes a badge (a round fill, lettering, a satin border), checks it, and
  writes a picture and a PES file that reads back; an argument that a
  tool does not take is refused; each change is one undo step; a change
  out of range is refused with its reason. The owner's test with an agent
  (a design from one sentence) is open.
- The agent works with the rights of the user: the tools read and write
  the files that the agent names. The server has no network code.
- The server has one design at a time, and only the bundled embroidery
  fonts.
