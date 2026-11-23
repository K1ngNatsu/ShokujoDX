// Measures the browser trial of ShokujoDX (roadmap P15, step 1; branch
// trial/browser) in a headless browser: the time to start, and the frame
// times of the canvas with synthetic designs (?stress=, ?look=, ?bench= of
// crates/emb-app/src/web.rs), the memory of the program, and a screenshot
// of each case.
//
// Use:  python tools/build-web.py --serve        (in one terminal)
//       node tools/web-bench.mjs [<browser> [<out folder>]]
//       (Node 22 or later; the browser: Edge on Windows if not given)

import { copyFileSync, mkdirSync, readdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { DEFAULT_BROWSER, openBrowser, sleep } from "./web-cdp.mjs";

const BROWSER = process.argv[2] ?? DEFAULT_BROWSER;
const OUT = process.argv[3] ?? "target/web-bench";
// BASE=<address>: another copy of the page (the test page on GitHub Pages).
const BASE = process.env.BASE ?? "http://localhost:8765/";
// [name, query, seconds]
const CASES = [
  ["open a project", "?open=two-color-patch.embproj&bench=still", 10],
  ["open, export DST", "?open=two-color-patch.embproj&export=dst", 10],
  ["lettering (Lato)", "?letter=Hamburgefonstiv&bench=still", 10],
  ["start page", "", 10],
  ["empty design, still", "?bench=still", 10],
  ["200k flat, still", "?stress=200000&look=flat&bench=still", 12],
  ["200k flat, still, WebGL2", "?stress=200000&look=flat&bench=still&gl=1", 12],
  ["200k 3D best, pan, WebGL2", "?stress=200000&look=best&bench=pan&gl=1", 20],
  ["200k flat, pan", "?stress=200000&look=flat&bench=pan", 12],
  ["200k flat, zoom", "?stress=200000&look=flat&bench=zoom", 12],
  ["200k 3D fast, pan", "?stress=200000&look=fast&bench=pan", 15],
  ["200k 3D good, pan", "?stress=200000&look=good&bench=pan", 20],
  ["200k 3D best, still", "?stress=200000&look=best&bench=still", 20],
  ["200k 3D best, pan", "?stress=200000&look=best&bench=pan", 20],
  ["200k 3D best, zoom", "?stress=200000&look=best&bench=zoom", 20],
  ["1M flat, pan", "?stress=1000000&look=flat&bench=pan", 20],
];

// WEBGPU=off: the browser has no WebGPU, so the program draws with WebGL2
// (as in Firefox and older Safari). ONLY=<text>: only the cases with that
// text in their name.
const NO_WEBGPU = process.env.WEBGPU === "off";
const ONLY = process.env.ONLY;
mkdirSync(OUT, { recursive: true });

const { send, evaluate, close, onEvent } = await openBrowser({
  browser: BROWSER,
  port: 9333,
  args: [
    "--window-size=1600,1000",
    ...(NO_WEBGPU ? ["--disable-features=WebGPU,Vulkan", "--disable-webgpu"] : ["--enable-unsafe-webgpu"]),
    // No frame cap of the headless browser: the frame times are the times
    // of the program.
    "--disable-gpu-vsync",
    "--disable-frame-rate-limit",
  ],
  name: "bench",
});
const logs = [];
onEvent((m) => {
  if (m.method === "Runtime.consoleAPICalled" || m.method === "Runtime.exceptionThrown") {
    const p = m.params;
    const text = p.args
      ? p.args.map((a) => a.value ?? a.description ?? "").join(" ")
      : JSON.stringify(p.exceptionDetails?.exception?.description ?? p.exceptionDetails?.text);
    logs.push(`${p.type ?? "exception"}: ${String(text).split("\n")[0].slice(0, 200)}`);
  }
});

// The test project next to the page, and the downloads in the out folder.
copyFileSync("packaging/test/two-color-patch.embproj", "target/web/two-color-patch.embproj");
await send("Browser.setDownloadBehavior", { behavior: "allow", downloadPath: join(process.cwd(), OUT) });
await send("Page.enable");
await send("Runtime.enable");
const results = [];
for (const [name, query, seconds] of CASES.filter(([n]) => !ONLY || n.includes(ONLY))) {
  logs.length = 0;
  await send("Page.navigate", { url: BASE + query });
  await sleep(seconds * 1000);
  const v = JSON.parse(
    await evaluate(`(async () => JSON.stringify({
      stats: window.shokujodxStats ? JSON.parse(window.shokujodxStats) : null,
      start_ms: window.shokujodxLoadMs ?? null,
      wasm_mb: window.shokujodxMemoryMb ? window.shokujodxMemoryMb() : null,
      webgpu: !!(navigator.gpu && (await navigator.gpu.requestAdapter())),
    }))()`),
  );
  const shot = await send("Page.captureScreenshot", { format: "png" });
  const file = name.replace(/[^a-z0-9]+/gi, "-").toLowerCase() + ".png";
  if (shot.result?.data) writeFileSync(join(OUT, file), Buffer.from(shot.result.data, "base64"));
  const problems = logs.filter((l) => /error|panic|exception/i.test(l));
  results.push({ name, ...v, problems });
  console.log(name.padEnd(22), JSON.stringify(v), problems.length ? `PROBLEMS: ${problems.slice(0, 3).join(" | ")}` : "");
}
writeFileSync(join(OUT, "results.json"), JSON.stringify(results, null, 2));
console.log("downloads:", readdirSync(OUT).filter((f) => !f.endsWith(".png") && f !== "results.json").join(", "));
await close();
