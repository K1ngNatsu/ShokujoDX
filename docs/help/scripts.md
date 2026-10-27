---
id: scripts
title: Scripts
group: reference
keywords: script, scripts, rhai, automation, macro, tool, batch, plugin, outline every fill, satin density, name threads, check before output, emb-cli script
---
# Scripts

A script is a small tool that changes the design, for example an outline
round each fill, or the same density for all satins. **Edit > Scripts**
runs the sample scripts of the program and your own scripts.

## Run a script

1. Select the objects (or select nothing: the script then gets all the
   design).
2. Click **Edit > Scripts** and choose a script. The window shows what it
   does and its settings.
3. Set the settings and click **Run**.

The messages of the script show under **Messages**. **Undo** takes back
all the changes of the script in one step. If the script stops with an
error, the design does not change.

A script can change only the design. It cannot open or save files, use
the network or start programs. A script that runs longer than 5 seconds
stops.

## Your own scripts

Scripts are text files with the extension `.rhai`, in the language Rhai
(see rhai.rs). Put them in the `scripts` folder of the settings folder:
**Open Scripts Folder** opens it, and **Copy to My Scripts** copies a
sample there for you to change. Click the round arrow to read the folder
again after you change a file.

The first lines that start with `//!` describe the script:

```rhai
//! Outline Every Fill
//! Adds a running outline round each fill.
//! input distance: 0.0 "Distance from the edge (mm)"
```

The first line is the title, the next lines are the description, and
each `input` line is a setting: a name, the default value (a number,
`true` or `false`, or a text in quotes) and the label. The script reads
the values as `input.distance`.

## Functions

Objects are numbers (`id`), threads are numbers (`t`). Sizes are in mm.

The design:

- `objects()`: all objects in sew order. `selection()`: the selected
  objects. `select([id, …])`: the selection after the script.
- `design_name()`, `set_design_name(text)`, `notes()`, `set_notes(text)`.
- `info()`: the numbers of the stitches: `stitches`, `colors`,
  `color_changes`, `trims`, `jumps`, `width_mm`, `height_mm`, `warnings`.

One object:

- `kind(id)`: `"fill"`, `"satin"`, `"running"`, `"applique"`,
  `"lettering"`, `"vector"`, `"stitches"`, `"cross_stitch"`, `"bling"` or
  `"sketch"`.
- `name(id)`, `set_name(id, text)`; `thread(id)`, `set_thread(id, t)`.
- `size(id)`: `x`, `y`, `width`, `height`.
- `settings(id)`: the stitch settings as a map, with the names of the
  project file (for example `spacing_mm` of a fill or a satin).
  `set_settings(id, map)` sets them:

  ```rhai
  let s = settings(id);
  s.spacing_mm = 0.4;
  set_settings(id, s);
  ```

- `sew_index(id)`, `move_to(id, index)`: the place in the sew order (0
  is the first).
- `remove(id)`.

More objects:

- `move_objects([id, …], dx, dy)`, `rotate_objects([id, …], degrees)`,
  `scale_objects([id, …], factor)` (about the center of the objects).
- `add_outline([id, …], distance)`: a running outline (out if the
  distance is more than 0, in if less). `add_border([id, …], width)`: a
  satin border outside. Both return the new objects.
- `shape([id, …], "weld")`: shaping (`weld`, `intersect`, `exclude`,
  `front_minus_back`, `back_minus_front`, `flatten`, `divide`, `combine`).
  It returns the new objects.

Threads:

- `threads()`: a list of maps with `id`, `name`, `color` (`"#rrggbb"`),
  `brand` and `code`.
- `add_thread(name, "#rrggbb")`: a new thread.
- `set_thread_name(t, text)`, `set_thread_color(t, "#rrggbb")`,
  `set_thread_code(t, brand, code)`.

Messages and errors:

- `print(text)`: a message in the window.
- `throw "text"`: stop with an error; the design does not change.

## From the command line

`emb-cli script my-tool.rhai design.embproj --out done` runs the script on
each project file and saves the changed files in the folder `done`.
`--set distance=1.5` sets an input. With no `--out`, it only prints the
messages, and nothing is saved.
