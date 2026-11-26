---
id: agents
title: Agents (MCP)
group: reference
keywords: agent, ai, assistant, mcp, model context protocol, tools, server, claude, chat, automation, emb-cli mcp, show, file changed, open again
---
# Agents (MCP)

An AI agent of your own (an assistant program that can use tools) can
make and change designs with ShokujoDX. The program has no AI in it and
uses no network: the agent starts the command-line tool `emb-cli` of the
program, and `emb-cli mcp` gives the agent the tools of the program
through the Model Context Protocol (MCP).

## Set up

Add a server to the MCP settings of your agent, with this command (the
folder of ShokujoDX, then `emb-cli mcp`):

```text
"C:\Program Files\ShokujoDX\emb-cli.exe" mcp
```

On a Mac the tool is in the program: `ShokujoDX.app/Contents/MacOS/emb-cli`.
On Linux it is `emb-cli` next to `shokujodx`. Many agents keep their
servers in a JSON file:

```json
{
  "mcpServers": {
    "shokujodx": {
      "command": "C:\\Program Files\\ShokujoDX\\emb-cli.exe",
      "args": ["mcp"]
    }
  }
}
```

## What the agent can do

The server keeps one design. Its tools:

- start a new design or open one (a project, a machine file or an SVG
  file), change its machine, hoop and fabric, and save it;
- list the objects, add shapes from SVG path text (runs, fills, satin
  columns and vector shapes) and add lettering in the embroidery fonts of
  the program;
- change the settings and the thread of objects, move, scale, rotate and
  delete them, and run a script (see [Scripts](scripts));
- check the design, make a picture of it with the 3D thread look, and
  export machine files, worksheets and approval sheets;
- make test swatches, make the design quicker to sew, and check and fix
  machine files;
- undo and redo.

Each change is one undo step. Measures are in mm from the middle of the
hoop. A tool that only reads says so, and a tool does not take an
argument that it does not know.

## See the design in the program

The tool **show** saves the design and opens it in ShokujoDX: in a new
tab of the program that runs, else in a new program.

When the agent saves a file that is open in a tab, the program asks
about it. **Open Again** shows the new version (the changes of the tab
that are not saved go). **Keep This Version** keeps the tab as it is.

## Try it

Ask your agent for a design in one sentence, for example: "Make a round
badge 60 mm wide with the word STAR in red and a blue satin border, and
show it in ShokujoDX."
