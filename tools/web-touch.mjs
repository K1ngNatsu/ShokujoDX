// Tests the browser version on touch screens (roadmap P15, step 3) in a
// headless browser with touch emulation: a tablet and a phone, the
// defaults of a first visit, and a pinch of two fingers on the canvas.
// It uses the addresses ?stress= and ?probe=1 of crates/emb-app/src/web.rs.
//
// Use:  python tools/build-web.py --serve        (in one terminal)
//       node tools/web-touch.mjs [<browser> [<out folder>]]
//       (BASE=<address>: another copy of the page)

import { mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { DEFAULT_BROWSER, STARTED, openBrowser, sleep } from "./web-cdp.mjs";

const BROWSER = process.argv[2] ?? DEFAULT_BROWSER;
const OUT = process.argv[3] ?? "target/web-touch";
const BASE = process.env.BASE ?? "http://localhost:8765/";
mkdirSync(OUT, { recursive: true });

// One browser with a new profile for each device, so each is a first visit.
async function device(name, width, height, scale, check) {
  const { send, evaluate, waitFor, close } = await openBrowser({
    browser: BROWSER,
    port: 9336,
    args: [`--window-size=${width},${height}`],
    name: "touch",
  });
  const view = async () => JSON.parse((await evaluate("window.shokujodxView ?? 'null'")) ?? "null");
  await send("Page.enable");
  await send("Emulation.setDeviceMetricsOverride", { width, height, deviceScaleFactor: scale, mobile: true });
  await send("Emulation.setTouchEmulationEnabled", { enabled: true, maxTouchPoints: 5 });
  await send("Page.navigate", { url: BASE + "?stress=20000&probe=1" });
  // The program runs and the design fits the view.
  await waitFor(STARTED);
  await sleep(1500);
  const coarse = await evaluate("matchMedia('(pointer: coarse)').matches");
  const result = { name, coarse, first: await view() };
  await check(send, view, result);
  const shot = await send("Page.captureScreenshot", { format: "png" });
  if (shot.result?.data) writeFileSync(join(OUT, `${name}.png`), Buffer.from(shot.result.data, "base64"));
  console.log(JSON.stringify(result));
  await close();
  return result;
}

// Two fingers 120 px apart in the middle of the canvas spread to 240 px and
// move 50 px down.
async function pinch(send, cx, cy) {
  const at = (t) => [
    { x: cx - 60 - 60 * t, y: cy + 50 * t, id: 0 },
    { x: cx + 60 + 60 * t, y: cy + 50 * t, id: 1 },
  ];
  await send("Input.dispatchTouchEvent", { type: "touchStart", touchPoints: at(0) });
  await sleep(120);
  for (let i = 0; i <= 10; i++) {
    await send("Input.dispatchTouchEvent", { type: "touchMove", touchPoints: at(i / 10) });
    await sleep(80);
  }
  await send("Input.dispatchTouchEvent", { type: "touchEnd", touchPoints: [] });
  await sleep(500);
}

// The device scale is 1: with an emulated scale of 2 the headless browser
// gave the program the canvas at half its size, so the touches missed it (a
// real phone and a real tablet do not do this).
const tablet = await device("tablet", 1180, 820, 1, async (send, view, r) => {
  const before = await view();
  await pinch(send, 590, 450);
  const after = await view();
  r.zoom_ratio = after && before ? +(after.zoom / before.zoom).toFixed(3) : null;
  r.undo_steps_added = after && before ? after.undo - before.undo : null;
  r.after = after;
});
const phone = await device("phone", 390, 844, 1, async () => {});

const problems = [];
if (!tablet.coarse) problems.push("the tablet has no coarse pointer");
if (!tablet.first || Math.abs(tablet.first.text_scale - 1.25) > 1e-6) problems.push("the tablet has no larger controls");
if (tablet.zoom_ratio == null || Math.abs(tablet.zoom_ratio - 2) > 0.25) problems.push(`the pinch zoomed ${tablet.zoom_ratio}`);
if (tablet.undo_steps_added !== 0) problems.push("the pinch made an undo step");
if (tablet.after?.gesture) problems.push("the gesture did not end");
if (!phone.first?.simple) problems.push("the phone is not in simple mode");
console.log(problems.length ? `PROBLEMS: ${problems.join("; ")}` : "touch: no problem");
