// The headless browser of the tests of the browser version (roadmap P15;
// tools/web-bench.mjs, web-touch.mjs, web-shots.mjs): it starts with a new
// profile, the tests talk to its page through the DevTools protocol, and
// it closes with all its processes.

import { spawn, spawnSync } from "node:child_process";
import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { basename, join } from "node:path";

export const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

// Edge on Windows, else Chrome.
export const DEFAULT_BROWSER =
  process.platform === "win32" ? "C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe" : "google-chrome";

// Starts the browser (`args`: more arguments) on the debugging port
// `port` and connects to its page. `name` goes in the name of the profile.
export async function openBrowser({ browser = DEFAULT_BROWSER, port, args = [], name = "page" }) {
  const profile = mkdtempSync(join(tmpdir(), `emb-web-bench-${name}-`));
  const child = spawn(
    browser,
    ["--headless=new", `--remote-debugging-port=${port}`, `--user-data-dir=${profile}`, "--no-first-run", ...args, "about:blank"],
    { stdio: "ignore" },
  );
  let list;
  for (let i = 0; i < 100 && !list; i++) {
    try {
      list = await (await fetch(`http://127.0.0.1:${port}/json/list`)).json();
    } catch {
      await sleep(200);
    }
  }
  if (!list) throw new Error("the browser did not start");
  const ws = new WebSocket(list.find((t) => t.type === "page").webSocketDebuggerUrl);
  await new Promise((r) => ws.addEventListener("open", r, { once: true }));
  let next = 0;
  const waiting = new Map();
  const listeners = [];
  ws.addEventListener("message", (e) => {
    const m = JSON.parse(e.data);
    if (m.id && waiting.has(m.id)) {
      waiting.get(m.id)(m);
      waiting.delete(m.id);
    } else if (m.method) {
      for (const f of listeners) f(m);
    }
  });
  const send = (method, params = {}) =>
    new Promise((r) => {
      const id = ++next;
      waiting.set(id, r);
      ws.send(JSON.stringify({ id, method, params }));
    });
  const evaluate = async (expression) =>
    (await send("Runtime.evaluate", { expression, returnByValue: true, awaitPromise: true })).result?.result?.value;
  // Waits until `expression` is true in the page, at most `ms`.
  const waitFor = async (expression, ms = 30000) => {
    for (const end = Date.now() + ms; Date.now() < end; await sleep(100)) {
      if (await evaluate(expression)) return true;
    }
    return false;
  };
  const close = async () => {
    ws.close();
    // The browser closes through its own connection (the connection of a
    // page does not close all its processes).
    try {
      const v = await (await fetch(`http://127.0.0.1:${port}/json/version`)).json();
      const b = new WebSocket(v.webSocketDebuggerUrl);
      await new Promise((r) => b.addEventListener("open", r, { once: true }));
      const exited = new Promise((r) => child.once("exit", r));
      b.send(JSON.stringify({ id: 1, method: "Browser.close" }));
      await Promise.race([exited, sleep(5000)]);
      b.close();
    } catch {
      // The browser is closed already.
    }
    // Then the profile goes. A process that still has it open stops first
    // (found by the name of the profile, never by the name of the program).
    for (let i = 0; i < 20; i++) {
      try {
        rmSync(profile, { recursive: true, force: true });
        return;
      } catch {
        if (i === 0) stopProcessesOf(profile);
        await sleep(500);
      }
    }
  };
  return { send, evaluate, waitFor, close, onEvent: (f) => listeners.push(f) };
}

function stopProcessesOf(profile) {
  if (process.platform === "win32") {
    spawnSync("powershell", [
      "-NoProfile",
      "-Command",
      `Get-CimInstance Win32_Process -Filter "Name='msedge.exe'" | Where-Object { $_.CommandLine -like '*${basename(profile)}*' } | ForEach-Object { Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue }`,
    ]);
  } else {
    spawnSync("pkill", ["-f", basename(profile)]);
  }
}

// The program has started in the page and the loader has gone (it fades
// out after the needle and the star are lit).
export const STARTED = "window.shokujodxLoadMs !== undefined && !document.getElementById('loading')";
