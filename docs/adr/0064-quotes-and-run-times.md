# ADR 0064: Quotes and run times

Date: 2026-10-27 · Status: Accepted (owner request, P13 Milestone 1)

## Context

A shop gives a customer a price before it sews an order, and it must know
how long the order takes on its machines. Before P13 the program gave the
sew time of one piece on one head (Machine Settings, ADR 0063 for the
thread changes) and the thread cost of one piece (ADR 0044). It did not
know machines with more heads, the time to hoop the pieces, or the prices
of the shop.

Shops price embroidery in many ways. The most common parts are a price for
each 1000 stitches, a lowest price for each piece, a price for colors over
a number, a setup price for each order, a price to digitize a new design,
lower prices for larger quantities, an added price for a rush order and
the tax. Commercial programs have price tools; their code is closed
(spec, rule 3). The method below is common shop practice, in our own
words.

## Decision

1. **Heads and hooping times are sew times.** `SewTimes` (for each
   machine, Design > Machine Settings) gets `heads` (1 to 50), `hoop_s`
   (hoop one piece and take it out after it sews), `frame_change_s` (take
   the hoops off and put the next hoops on) and `hoop_while_sewing` (a
   second set of hoops). A machine profile can have `heads`; the built-in
   profiles have none, because one model is sold with different numbers
   of heads. The user sets the heads.
2. **Run time** (`emb_model::quote::run_time`). One cycle sews one piece
   on each head. The operator hoops the first pieces before the start;
   with a second set of hoops the next pieces are hooped while the machine
   sews, so a cycle takes the longer of the sew time and the hoop time;
   else the times add. Each cycle ends with a frame change. The result
   gives the cycles, the time, the pieces in an hour, and if the operator
   or the machine sets the speed. The sew time of one piece is the time of
   Design info (with the stops and the thread changes).
3. **Price lists** (`emb_model::quote::PriceList`) are user settings
   (`Settings::price_lists`, one list "Standard" with no prices at the
   start), with the parts above. The currency is the currency of the
   thread prices (Options), so the program has one currency.
4. **The quote** (`emb_model::quote::quote`) is a list of lines in a fixed
   order: stitches, added colors, the top-up to the lowest price, the
   quantity price (a percent off the embroidery price, from the break with
   the largest quantity not over the order), garments, setup, digitizing
   (new designs only), rush (a percent of the sum), tax (a percent of the
   sum with the rush price). Each line is in cents, and the price of each
   piece is in cents before it is multiplied, so the printed lines add up
   to the total. Bad values (negative, not finite, percents over 100) are
   cleaned before the quote. The colors are the different threads
   (`needles::same_thread`), not the color blocks.
5. **Where the user sees it.** The Quote window (Design tab): the price
   list and its prices, the order (quantity, new design, rush, garment
   price), the lines, the total and the price of each piece, the thread
   cost of each piece, and the run time. Design info shows the price of
   each piece and the run time of the quote quantity. The quote sheet
   (PDF, for the customer) has the design picture, the lines and the
   total. `emb-cli quote` gives the same numbers from a price list file
   (TOML). The order of the Quote window is window state in Milestone 1;
   Milestone 2 keeps it in the design (`Design::order`).

## Consequences

- The price is an estimate that the shop can change: the program does
  not save quotes or know customers (P13, "Not in this phase").
- A machine that sews different designs on its heads, or two machines on
  one order, is not in the run time. The user can divide the order.
- The settings file gets `price_lists`, and the sew times of each machine
  get four values (old settings files get the defaults).
