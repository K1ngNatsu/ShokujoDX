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
    `tools/list` and `tools/call`. The results of `server/discover`,
    `tools/list` and `tools/call` have `resultType` `complete` (2026-07-28
    needs it; the versions before it do not read it).
  - A message that is not JSON, or a line that is not UTF-8 text, is
    error -32700; a batch, JSON that is not an object, and a request with
    no method are -32600; an unknown method is -32601, an unknown tool
    -32602. The session goes on after each of them. A reply of the client
    (a result or an error) gets no reply. An error of a tool is a result
    with `isError` true and the reason, so that the agent can correct its
    call; a fault (a panic) in a tool is such an error too, and the
    session and its design stay.
  - A result has the value of the tool as JSON text and as
    `structuredContent`; the picture is also an image (PNG in base64).
- **One design.** The server keeps one design (`emb_model::Document` with
  its history). Each change is one `Document::edit`: one undo step. A
  change that the document refuses (ADR 0139) gives its reason
  (`Document::take_refused`). `new_design` and `open_design` do not take
  the place of a design with objects and changes that are not saved
  unless `discard_changes` is true (a design with no objects goes). A
  move of 0 or a scale of 1 is no change (no undo step), and an id that
  a call gives two times is one object.
- **26 tools:** `new_design`, `set_design` (name, machine, hoop and
  fabric), `open_design`, `save_design`, `design_info`, `list_objects`, `add_shape`, `add_lettering`,
  `set_settings`, `set_thread`, `move_objects`, `scale_objects`,
  `rotate_objects`, `delete_objects`, `undo`, `redo`, `run_script`,
  `check_design`, `picture`, `export`, `test_swatches`, `quicker_to_sew`
  (with `dry_run`), `check_machine_file`, `fix_machine_file`, `show` and
  `list_catalog`. Measures are mm from the middle of the hoop, x to the
  right and y down; angles are degrees. The server instructions and the
  descriptions of the tools that take coordinates say so (a client can
  leave the instructions out).
- **Boxes** are the boxes of the curves, not of their control points
  (the boxes of objects; the size of the design in `design_info` is the
  size that the program shows).
  Lettering that is laid out again (a new text or size) keeps the middle
  of its box; `move_objects` also puts the middle of a box at x, y. Each
  object in a result says if its shape fits the hoop.
- **Checks for the hoop.** `check_design` warns about the stitches of each
  object outside the field of the hoop (as Design info in the program),
  `design_info` says if the design fits the hoop, and
  `check_machine_file` gives the size, stitches, colors, trims and
  threads of a file (the colors that the file keeps), and if it fits a
  hoop with the middle of its stitches at the middle of the hoop, as the
  machine puts it (the origin of a file can be its first stitch or a
  corner; the audit of 2026-11-26).
- **Checked arguments.** The schema of each tool has
  `additionalProperties: false`. A tool refuses an argument that it does
  not take and names the ones that it takes, an argument that it needs
  and does not get, and a value of the wrong type (numbers must be real
  numbers). A tool that only reads has `readOnlyHint`, and its
  description ends with "It only reads." A tool that can take away work
  (a design, objects, a file that it writes over: `picture` with a path,
  `save_design`, `export`, `test_swatches`, `fix_machine_file`) has
  `destructiveHint`. `picture` writes only a `.png` file. The text of
  lettering has at most 2,000 characters (`MAX_TEXT_CHARS`: a text of
  1 MB kept the server busy for more than 30 s).
- **Shapes** come from SVG path text in mm: the SVG reader of the program
  reads it (all the commands of SVG, also arcs) with a viewBox of mm, and
  the coordinates are rounded to 0.1 µm (the reader keeps 32-bit
  numbers). A fill takes the first closed part as its outline and the
  others as holes; a satin column takes the path as its middle line.
- **Settings.** `set_settings` changes the settings of objects by the
  names that `list_objects` shows (the saved form of the settings; the
  font of lettering also by its name, as `add_lettering` takes it). A
  group of settings in a change (for example `satin`) goes into the group
  that is there; a change of its kind (`mode`) takes its place. A
  name that is not a setting is refused (a setting at its default value
  is not written, so the server tries values to find it). Lettering is
  laid out again with its bundled embroidery font; the server has only
  the bundled embroidery fonts.
- **Results that say what changed.** `quicker_to_sew` gives the merged
  threads, the removed objects and the trim distance before and after,
  and makes no undo step when nothing changes. `export` makes the project
  that it writes the file of the design. A picture shows the fabric of
  the design (its weave in its color, else white), a plain color, or
  nothing (`none`).
- **Show.** `show` saves the design to its file, else to a file of the
  session in the folder `ShokujoDX agent` of the temporary files (a clean
  name of the design and the number of the process; the design stays not
  saved), and starts the program next to `emb-cli` with the file
  (`SHOKUJODX_PROGRAM` names another program, for the tests). That start
  gives the file to the program that runs (the inbox of ADR 0175; a file
  that is open already shows its tab) or opens a window. The output of
  the program does not go to the output of the server. On Windows the
  program starts through PowerShell `Start-Process`, with the paths in
  environment variables: a process that Rust starts gets each handle of
  `emb-cli` that a child can take, also the pipes of the client, so the
  client saw no end of the output of the server while the program was
  open (measured: 15 s for a program of 15 s; through the shell 0 s).
- **A file that another program saves.** The program knows the time of
  each file that it opened or saved (`file_watch.rs`, `file_known`). Each
  second it compares the file of the tab in front (`poll_file_changes`).
  A file that changed gets the question File Changed: Open Again puts the
  design of the file in the tab with a new history and a new id
  (`AppState::replace_tab_design`), and Keep This Version keeps the tab
  (not saved: a close asks). The files that a job of the program writes
  (Output saves the project of the tab) are the program's own
  (`files_written`), and no question comes while a job runs.
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
  out of range is refused with its reason (about 3 s, most of it to read
  the 68 fonts for `list_catalog`). The owner's test with an agent (a
  design from one sentence) is open.
- An agent test with subagents (2026-11-25): three agents, each with only
  the tool list, made a round badge, a name with a heart for a Brother
  machine, and a square patch with a swatch and a DST file, through a
  bridge to the server. All three designs were right. Their friction made
  the changes above (boxes, the middle of lettering, the hoop checks,
  `set_design`, the background, the results of Quicker to Sew and
  export). A fourth agent checked the changes. The labels of test
  swatches at 5 mm sewed heavy: they are Soft Round now (ADR 0170). One
  finding is open (`docs/design/open-work.md`): the travel measure does
  not count a short travel stitch between two letters on a fill (under
  the limit of a warning).
- The agent works with the rights of the user: the tools read and write
  the files that the agent names. The server has no network code.
- The server has one design at a time, and only the bundled embroidery
  fonts.
