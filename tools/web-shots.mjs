// Screenshots of the browser version on phones and tablets (roadmap P15,
// step 3), in a headless browser with touch emulation: the layout in
// portrait and in landscape, with the settings of a user who chose the
// full mode and a machine (no first start question).
//
// Use:  python tools/build-web.py --serve        (in one terminal)
//       node tools/web-shots.mjs [<browser> [<out folder>]]
//       (BASE=<address>: another copy of the page)

import { mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { DEFAULT_BROWSER, STARTED, openBrowser, sleep } from "./web-cdp.mjs";

const OUT = process.argv[3] ?? "target/web-shots";
const BASE = process.env.BASE ?? "http://localhost:8765/";
mkdirSync(OUT, { recursive: true });
const { send, evaluate, waitFor, close } = await openBrowser({
  browser: process.argv[2] ?? DEFAULT_BROWSER,
  port: 9338,
  args: ["--window-size=1200,1200"],
  name: "shots",
});

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
    "?stress=20000&probe=1",
    [
      // A swipe on the tab row shows the tabs at its end.
      ["phone-portrait-tabs-swiped", () => finger(330, 13, 40, 13)],
      // A tap on the button at the edge of the canvas hides the side panels
      // (upright: at the bottom edge; the place of the canvas from ?probe=1).
      [
        "phone-portrait-side-hidden",
        async () => {
          const [x0, , x1, y1] = JSON.parse(await evaluate("window.shokujodxView")).canvas;
          await finger((x0 + x1) / 2, y1 - 15, (x0 + x1) / 2, y1 - 15);
        },
      ],
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

await send("Page.enable");
await send("Emulation.setTouchEmulationEnabled", { enabled: true, maxTouchPoints: 5 });
// The settings in the storage of the page, before the cases.
await send("Page.navigate", { url: BASE + "index.html?seed" });
await waitFor("document.readyState === 'complete'");
await evaluate(`localStorage.setItem("shokujodx-settings", ${JSON.stringify(JSON.stringify(SETTINGS))})`);
async function picture(name) {
  const shot = await send("Page.captureScreenshot", { format: "png" });
  if (shot.result?.data) writeFileSync(join(OUT, `${name}.png`), Buffer.from(shot.result.data, "base64"));
  console.log("wrote", join(OUT, `${name}.png`));
}
for (const [name, width, height, query, steps] of CASES) {
  // The device scale is 1 (see tools/web-touch.mjs).
  await send("Emulation.setDeviceMetricsOverride", { width, height, deviceScaleFactor: 1, mobile: true });
  await send("Page.navigate", { url: BASE + query });
  // The program runs, then a moment for the view to settle.
  await waitFor(STARTED);
  await sleep(1500);
  await picture(name);
  for (const [step, action] of steps) {
    await action();
    await picture(step);
  }
}
await close();
