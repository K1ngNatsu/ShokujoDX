# ADR 0066: Thread stock

Date: 2026-10-27 · Status: Accepted (owner request, P13 Milestone 3)

## Context

A shop has a shelf of threads. It wants designs in the threads that it
has, and it must know which threads to buy before an order. Before this
change the program matched colors to a full brand chart only, and it did
not know the threads of the shop.

## Decision

1. **The stock is a user setting** (`Settings::thread_stock`, an
   `emb_model::stock::ThreadStock`): for each thread the brand, code,
   name, color, the spools on hand, the length of a spool (else the spool
   length of the thread prices, ADR 0044) and the lowest number of spools
   to keep. A thread is the same as a design thread if the brand and code
   agree (case is ignored), or, with no code, the color and name. CSV
   import and export move the stock between computers.
2. **The stock is a thread chart.** The threads with at least one spool
   make the chart "Threads in stock", last in the chart list. A chart
   thread can have its own brand (`ChartThread::brand`), so the looks of
   the stock keep their brands. **Only threads in stock** shows this
   chart. All color matching of the program takes the chart that shows
   (Match Design to Chart, the picture tools, Picture to Design and its
   cross stitch step), so they use only the threads in stock with no
   code of their own. Turning it off shows the chart that showed before.
3. **Use Threads in Stock** (`Design::use_stock_threads`) gives each used
   thread of the current colorway that is not in stock the look of the
   nearest thread in stock (CIE76), in one undo step, and reports each
   change with its color difference.
4. **Threads to buy** (`emb_model::stock::threads_to_buy`): for the
   quantity of the order (ADR 0065), the top thread of each different
   thread of the design (`thread_use`, ADR 0044) against the stock. A
   thread is on the list if it is not in stock, if the order needs more
   than the stock has, or if fewer spools than the lowest number are left
   (a spool that is begun counts as one). The list gives whole spools. The
   bobbin thread is not on the list: a shop uses one or two bobbin colors
   for all designs.
5. **Where the user sees it.** The Thread Stock window (Design > Orders):
   add threads from a chart, the stock list, the threads of the design
   with the nearest thread in stock, Use Threads in Stock, the threads to
   buy and their CSV. The Threads panel marks the threads in stock (●) and
   has the Only threads in stock switch. Design info gives the number of
   threads to buy, and the production worksheet lists them. Nothing
   changes for a user with no stock.

## Consequences

- The numbers of the stock change only when the user changes them: the
  program does not take thread out of the stock when a design sews.
- Thread use is an estimate (ADR 0044), so the list of threads to buy is
  an estimate too.
- The settings file gets `thread_stock`.
