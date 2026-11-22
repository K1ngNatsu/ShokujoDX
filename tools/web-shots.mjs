// Screenshots of the browser version on phones and tablets (roadmap P15,
// step 3), in a headless browser with touch emulation: the layout in
// portrait and in landscape, with the settings of a user who chose the
// full mode and a machine (no first start question).
//
// Use:  python tools/build-web.py --serve        (in one terminal)
//       node tools/web-shots.mjs [<browser> [<out folder>]]
//       (BASE=<address>: another copy of the page)

import { spawn, spawnSync } from "node:child_process";
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { basename, join } from "node:path";

const BROWSER =
  process.argv[2] ??
  (process.platform === "win32"
    ? "C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe"
    : "google-chrome");
const OUT = process.argv[3] ?? "target/web-shots";
const BASE = process.env.BASE ?? "http://localhost:8765/";
const PORT = 9338;
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
mkdirSync(OUT, { recursive: true });

// The settings of a user who chose the full mode and a machine.
const SETTINGS = { simple_mode: false, machine: "generic", text_scale: 1.0 };

// A drag of one finger from (x0, y0) to (x1, y1), or a tap if they are
// the same point.
async function finger(x0, y0, x1, y1) {
  await send("Input.dispatchTouchEvent", { type: "touchStart", touchPoints: [{ x: x0, y: y0, id: 0 }] });
  await sleep(80);
  for (let i = 1; i <= 8 && (x0 !== x1 || y0 !== y1); i++) {
    const t = i / 8;
    await send("Input.dispatchTouchEvent", {
      type: "touchMove",
      touchPoints: [{ x: x0 + (x1 - x0) * t, y: y0 + (y1 - y0) * t, id: 0 }],
    });
    await sleep(40);
  }
  await send("Input.dispatchTouchEvent", { type: "touchEnd", touchPoints: [] });
  await sleep(1500);
}

// [name, width, height, query, steps after the first picture: [name, action]]
const CASES = [
  ["phone-portrait-start", 390, 844, "", []],
  [
    "phone-portrait-design",
    390,
    844,
    "?stress=20000",
    [
      // A swipe on the tab row shows the tabs at its end.
      ["phone-portrait-tabs-swiped", () => finger(330, 13, 40, 13)],
      // A tap on the button at the edge of the canvas hides the side panels.
      ["phone-portrait-side-hidden", () => finger(199, 493, 199, 493)],
    ],
  ],
  ["phone-landscape-start", 844, 390, "", []],
  [
    "phone-landscape-design",
    844,
    390,
    "?stress=20000",
    // A tap on a tab shows its panels over the canvas.
    [["phone-landscape-peek", () => finger(310, 13, 310, 13)]],
  ],
  ["tablet-portrait-design", 820, 1180, "?stress=20000", []],
  [
    "phone-menus",
    390,
    844,
    "?stress=20000",
    [
      // A long press on the top ruler: the menu of the ruler.
      ["phone-ruler-menu", () => hold(120, 202)],
      ["phone-ruler-closed", () => key("Escape")],
      // File: the menu with Check for Updates.
      ["phone-file-menu", () => finger(34, 13, 34, 13)],
      // Check for Updates…: the window with the build of the page.
      ["phone-update-window", () => finger(95, 646, 95, 646)],
    ],
  ],
];

// A finger that stays down (a long press).
async function hold(x, y) {
  await send("Input.dispatchTouchEvent", { type: "touchStart", touchPoints: [{ x, y, id: 0 }] });
  await sleep(1400);
  await send("Input.dispatchTouchEvent", { type: "touchEnd", touchPoints: [] });
  await sleep(1200);
}

async function key(name) {
  for (const type of ["keyDown", "keyUp"]) {
    await send("Input.dispatchKeyEvent", { type, key: name, code: name, windowsVirtualKeyCode: name === "Escape" ? 27 : 0 });
  }
  await sleep(800);
}

async function closeBrowser(profile) {
  try {
    const v = await (await fetch(`http://127.0.0.1:${PORT}/json/version`)).json();
    const b = new WebSocket(v.webSocketDebuggerUrl);
    await new Promise((r) => b.addEventListener("open", r, { once: true }));
    b.send(JSON.stringify({ id: 1, method: "Browser.close" }));
    await sleep(1500);
    b.close();
  } catch {
    // The browser is closed already.
  }
  if (process.platform === "win32") {
    spawnSync("powershell", [
      "-NoProfile",
      "-Command",
      `Get-CimInstance Win32_Process -Filter "Name='msedge.exe'" | Where-Object { $_.CommandLine -like '*${basename(profile)}*' } | ForEach-Object { Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue }`,
    ]);
  }
  for (let i = 0; i < 20; i++) {
    try {
      rmSync(profile, { recursive: true, force: true });
      return;
    } catch {
      await sleep(500);
    }
  }
}

const profile = mkdtempSync(join(tmpdir(), "emb-web-bench-shots-"));
spawn(
  BROWSER,
  [
    "--headless=new",
    `--remote-debugging-port=${PORT}`,
    `--user-data-dir=${profile}`,
    "--window-size=1200,1200",
    "--no-first-run",
    "about:blank",
  ],
  { stdio: "ignore" },
);
let list;
for (let i = 0; i < 100 && !list; i++) {
  try {
    list = await (await fetch(`http://127.0.0.1:${PORT}/json/list`)).json();
  } catch {
    await sleep(200);
  }
}
const ws = new WebSocket(list.find((t) => t.type === "page").webSocketDebuggerUrl);
await new Promise((r) => ws.addEventListener("open", r, { once: true }));
let next = 0;
const waiting = new Map();
ws.addEventListener("message", (e) => {
  const m = JSON.parse(e.data);
  if (m.id && waiting.has(m.id)) {
    waiting.get(m.id)(m);
    waiting.delete(m.id);
  }
});
const send = (method, params = {}) =>
  new Promise((r) => {
    const id = ++next;
    waiting.set(id, r);
    ws.send(JSON.stringify({ id, method, params }));
  });
await send("Page.enable");
await send("Emulation.setTouchEmulationEnabled", { enabled: true, maxTouchPoints: 5 });
// The settings in the storage of the page, before the cases.
await send("Page.navigate", { url: BASE + "index.html?seed" });
await sleep(2000);
await send("Runtime.evaluate", {
  expression: `localStorage.setItem("shokujodx-settings", ${JSON.stringify(JSON.stringify(SETTINGS))})`,
});
async function picture(name) {
  const shot = await send("Page.captureScreenshot", { format: "png" });
  if (shot.result?.data) writeFileSync(join(OUT, `${name}.png`), Buffer.from(shot.result.data, "base64"));
  console.log("wrote", join(OUT, `${name}.png`));
}
for (const [name, width, height, query, steps] of CASES) {
  // The device scale is 1 (see tools/web-touch.mjs).
  await send("Emulation.setDeviceMetricsOverride", { width, height, deviceScaleFactor: 1, mobile: true });
  await send("Page.navigate", { url: BASE + query });
  await sleep(8000);
  await picture(name);
  for (const [step, action] of steps) {
    await action();
    await picture(step);
  }
}
ws.close();
await closeBrowser(profile);
