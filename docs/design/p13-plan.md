# Plan: roadmap phase P13 (Shop tools)

Date: 2026-10-27 · Status: Accepted (owner request, 2026-10-27). Milestones 1 to 4 done in software; the exit tests are open.

## Goal

P1 to P12 give a shop the tools to digitize and sew a design. A shop must
also sell the work, plan it and keep thread for it. A review on
2026-10-27 found four gaps that a professional user has every day and
that no earlier phase plans:

1. **Quotes and run time.** The price of an order (stitches, setup,
   quantity), and the time to sew it on machines with more heads. Now the
   program gives the thread cost and the sew time of one piece on one
   head only (P3, P11 Milestone 1).
2. **Order details.** The customer, the order number, the garment, the
   placement, the quantity and the due date on the worksheet and the
   approval sheet. Now the sheets have the design notes and a barcode of
   the file name only (P3, P8 Milestone 7).
3. **Thread stock.** The threads that the shop has, so that color
   matching uses only them, and a list of the threads to buy for an
   order.
4. **Scripts.** Tools that a user writes for the work of the shop, in the
   program and in `emb-cli`, with no access to the computer outside the
   design.

The methods are common shop practice, in our own words. Commercial
programs have quotes and production tools; their code is closed
(spec, rule 3). This phase is not a shop management system: it has no
customer database, no invoices and no accounts (see "Not in this
phase").

## What we have

- `emb_model::machine::SewTimes` and `SewTimes::minutes`: the sew time
  of one piece (speed, color changes, trims, stops), for each machine
  (Design > Sew Times).
- `emb_model::thread_use` and `ThreadPrices` (Options): the top and
  bobbin thread of each color, and its cost.
- `emb_model::needles` (ADR 0063): the thread changes of a design on a
  multi-needle machine.
- `emb_render::sheet`: the worksheet and the approval sheet, with a
  barcode (Code 39, Code 128).
- `emb_art::reduce::match_threads` and `reduce_with`: color matching to a
  thread chart (P6, P12 Milestone 3).
- `emb-cli`: batch output, `stitch`, `output`.

## Order

Milestone 1 comes first: the quote needs the run time, and Milestone 2
puts the quote on the sheets. Milestone 3 uses the quantity of
Milestone 2 for the threads to buy. Milestone 4 can use all of them.

| # | Scope | Size | Exit test |
| --- | --- | --- | --- |
| 1 | **Quotes and run time.** (a) Machines with more heads: the number of heads in the machine profile (`machines.toml`, unverified data as for the needles) and for each user (Design > Sew Times). (b) Run time of an order: a quantity, the heads, the time to hoop one piece and the time to change the frames. One cycle sews one piece on each head. The operator hoops the next pieces while the machine sews, so a cycle is the longer of the sew time and the hoop time of all heads, plus the frame change. The result: cycles, run time, pieces in an hour, and which one sets the speed (the machine or the operator). (c) A price list in the settings (`emb_model::quote::PriceList`): a price for each 1000 stitches, a minimum price for each piece, a setup price for each order, a digitizing price (a fixed price or a price for each 1000 stitches, once for each new design), an added price for each color over a number, quantity breaks (from a quantity, a percent off), an added percent for a rush order, the tax percent and the currency text. More than one price list (for example "Shirts" and "Caps"). (d) The quote (`emb_model::quote::quote`): each line of the price, the price of one piece and of the order, the thread cost (P11) and the run time; the garment price that the user types goes in as a line. (e) The Quote window (Design tab), the quote in Design info, a quote sheet (PDF) for the customer, and `emb-cli quote`. | M | Unit tests: the quote and the run time of test orders agree with values found by hand (each line, quantity breaks, minimum price, operator-bound and machine-bound cycles); the Quote window draws for each corpus design (`smoke.rs`); a shop tester compares the quote and the run time of three real orders with their own numbers. |
| 2 | **Order details.** (a) `Design::order` (`emb_model::order::OrderInfo`): customer, order number, garment, garment color, sizes and quantities (for example S 10, M 20), placement (left chest, cap front, back and others, with the product places of P7 Milestone 5), due date, price list and notes for the order. Old projects open with no order. (b) The Order window (Design tab), and the order on the worksheet and the approval sheet: the order number as a barcode (in place of the file name when there is one), the placement on the product picture, and on the approval sheet the price of the quote and a line to sign (name and date). (c) Output design uses the order number in the file names (an option of the output profile), and the design properties CSV has the order fields. (d) A template keeps no order (a new design from a template starts with an empty order). | S | Unit tests: the order round trip in the project file; the sheets show the fields; an old project opens; the barcode of the order number reads back with a phone app; a tester prints a sheet for a real order. |
| 3 | **Thread stock.** (a) The stock (`Settings::thread_stock`, `emb_model::stock`): for each thread (brand chart and code) the number of spools, the length of a spool (from the chart, or a value the user types) and a lowest number to keep. Import and export as CSV. (b) The Thread Stock window (Design tab): add threads from the charts, change the numbers, search. The Threads panel and the thread picker mark the threads in stock. (c) "Only threads in stock": an option of the color matching (Threads panel, Picture to design, picture to cross stitch, bitmap colors); "Use Threads in Stock" changes each thread of the design to the nearest thread in stock and shows the color difference (ΔE) of each change. (d) Threads to buy: for the quantity of the order (Milestone 2), the thread use of each color (P11) against the stock; a list of the threads that are not in stock or that go under the lowest number. In Design info, on the worksheet, and as CSV. | M | Unit tests: the nearest thread in stock; the threads to buy for a test order; the CSV round trip; Picture to design with "Only threads in stock" gives only threads in stock. |
| 4 | **Scripts.** (a) A new engine crate `emb-script` (an ADR decides the language: a small script language in pure Rust, such as Rhai, or WebAssembly through a pure-Rust interpreter). (b) What a script can do: read the design (objects, kinds, threads, settings, sew order, the stitch plan numbers of Design info), and change it through one call of `Document::edit` (one undo step): add, remove, move and change objects and threads, and run the commands of the engine (shaping, offsets, sew order). (c) What a script cannot do: open files, use the network or start programs. The program gives a script only the design and the values that the user types in a form that the script asks for. A script that runs too long or uses too much memory stops with a message, and the design does not change. (d) The Scripts panel (Edit tab): the scripts of the user folder and the bundled sample scripts, run with the selection, and the messages of a script. `emb-cli script` runs a script on project files (batch work). (e) Sample scripts: a run border round each fill, all satins to one density, threads renamed from a chart, and a check of the order before output. (f) Help topic for script writers, with the API. | L | Unit tests: each sample script gives the expected design (golden snapshots); a script with no end stops; a script that tries to open a file gets an error; undo takes back a script in one step; `emb-cli script` runs on the corpus. |

Then the exit test of the phase: the exit tests of each milestone pass.

## Not in this phase

| Item | Reason |
| --- | --- |
| A customer database, order tracking, invoices, payments, accounts | These are a shop management system, not a digitizer. The order details, the quote sheet and the CSV files give the data to such a system. |
| Prices from a supplier, ordering thread on line | No supplier publishes a protocol, and prices change. The user types the prices (Options). |
| Scripts with access to files, the network or other programs | A script from another person must not harm the computer. Revisit if users need it, with a clear permission for each script. |
| A shared stock for more computers | The settings are for one user. The CSV export and import move the stock. |

## Progress

| # | State | Notes |
| --- | --- | --- |
| 1 | Done in software | ADR 0064. `emb_model::quote`: `PriceList` (cleaned before use; `from_toml_str` refuses unknown fields), `Order`, `Piece` (the different threads, `thread_count`), `quote` (lines in cents: stitches, added colors, top-up to the lowest price, quantity price, garments, setup, digitizing, rush, tax), `run_time` (cycles of one piece on each head, hooping of the next pieces while the machine sews or not, a frame change for each cycle; machine-bound or operator-bound). `SewTimes` has `heads`, `hoop_s`, `frame_change_s`, `hoop_while_sewing`; profiles can have `heads` (none set: a model is sold with different numbers of heads). App: Design > Orders > Quote (`emb-app/src/quote.rs`; `Settings::price_lists`, the order is window state until Milestone 2), the four times in Machine Settings, the price of each piece and the run time in Design info, the quote sheet (`emb_render::sheet::quote_sheet`), help topic `quotes`. `emb-cli quote` with `--prices`, `--quantity`, `--heads`, `--sheet`. Golden snapshots: no change. Not done: the exit test with a shop tester (three real orders). |
| 2 | Done in software | ADR 0065. `emb_model::order::OrderInfo` in `Design::order` (customer, number, garment and color, sizes or a quantity, placement, due date, price list name, new design, rush, garment price, notes; `rows`, `total_quantity`, `quote_order`, `file_part`); no `order` field when there is none; templates drop it. Order Details window (`emb-app/src/order.rs`; placement menu from the product places, a due-date check, usual sizes); the Quote window keeps its order in the design (`AppState::set_order`, one undo step for each field). Sheets: order rows on the worksheet, approval sheet (with the price, `Catalogs::order_price`) and quote sheet; order notes after the design notes; the barcode has the order number (`SheetData::barcode_text`); six CSV columns at the end. Output profile: order number before the file names. Design info: the order number and customer. `emb-cli quote` takes the order of the design. Not done: the placement is text, not drawn on a product picture; the exit test (a barcode read with a phone; a tester prints a sheet for a real order). |
| 3 | Done in software | ADR 0066. `emb_model::stock`: `ThreadStock` (`Settings::thread_stock`; brand and code, or color and name, find a thread; CSV import and export), `as_chart` (the chart "Threads in stock"; `ChartThread::brand` keeps the brand of each thread), `nearest`, `Design::use_stock_threads` (one undo step, with ΔE), `thread_lengths` and `threads_to_buy` (not in stock, not enough, under the lowest number after the order; whole spools; no bobbin thread), `buy_csv`. App: Design > Orders > Thread Stock (`emb-app/src/stock.rs`); Only threads in stock shows the stock chart, so all color matching (Threads panel, Match Design to Chart, picture tools, Picture to Design) uses it; marks (●) in the Threads panel; threads to buy in Design info and on the worksheet (`SheetData::to_buy`). Help topic `thread-stock`. Not done: an import from the files of a supplier (each supplier has its own columns: use the CSV of the program). |
| 4 | Done in software | ADR 0067: Rhai (pure Rust, no file, network or process functions). New engine crate `emb-script`: `run` on a copy of the design (the app puts the result in with one edit), `check`, `ScriptInfo::parse` (title, description and inputs from `//!` lines), `Limits` (5 s, 50 million operations, call depth, string, array and map sizes, 20,000 objects). Functions: objects, selection, kind, name, thread, size, settings and set_settings (the serde map of the stitch settings; whole numbers become decimal numbers where needed), sew order, remove, move, rotate, scale, threads and their looks, add_outline, add_border, shape, info, print. Samples in `data/scripts` (Outline Every Fill, Same Density for All Satins, Name Threads by Code, Check Before Output) with snapshot tests; tests for the limits and the sandbox. App: Edit > Scripts (`emb-app/src/scripts.rs`; the samples and the `scripts` folder of the settings folder, the input form, Run, the messages, Copy to My Scripts). `emb-cli script` with `--set` and `--out` (with no `--out`, nothing is saved). Help topic `scripts` with the function list. The script runs on the UI thread (at most 5 s). |
