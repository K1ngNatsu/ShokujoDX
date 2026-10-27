---
id: thread-stock
title: Thread stock
group: production
keywords: thread stock, inventory, spools, threads in stock, buy, order thread, reorder, lowest, minimum, shopping list, color matching, nearest thread, delta e
---
# Thread stock

**Design > Thread Stock** keeps the threads that you have. The program
can then match colors with only these threads, change the threads of a
design to the nearest threads in stock, and list the threads to buy for
an order.

## Add your threads

- **Add from a chart**: choose a thread chart, type a name or code, and
  click **Add** next to a thread. Each click adds one spool.
- **Import CSV**: a file with the columns `brand,code,name,color,spools,spool_m,min_spools`
  (the color as `#rrggbb`). The last three columns can be left out: one
  spool, the spool length of Options, no lowest number. **Export CSV**
  writes the same columns, for example to move the stock to another
  computer.

In the list, set for each thread:

- **Spools**: the spools that you have. Count a spool that you began as
  one.
- **Spool length**: the thread on one spool. 0 uses the length of the
  thread prices in Options.
- **Lowest**: the lowest number of spools to keep. Under it, the thread
  goes on the list of threads to buy.

## Only threads in stock

**Only threads in stock** (in the Thread Stock window and in the Threads
panel) makes the stock the thread chart: the chart list, **Match Design
to Chart**, the picture tools and **Picture to Design** then use only the
threads that you have. Turn it off to go back to the chart that showed
before.

The Threads panel marks a thread in stock with a green dot (●).

## Threads of this design

The window lists each different thread of the design: in stock (with the
spools), or not in stock with the nearest thread in stock and the color
difference (ΔE). Under about 2.3 a person cannot see the difference; over
about 10 the colors are clearly different.

**Use Threads in Stock** gives each used thread that is not in stock the
nearest thread in stock. Undo takes back all the changes in one step.

## Threads to buy

For the quantity of the order (**Design > Order Details**), the program
compares the top thread that the order needs (from Design info) with the
stock. A thread is on the list if it is not in stock, if the order needs
more than you have, or if fewer spools than the lowest number are left
after the order. The list gives the spools to buy. The bobbin thread is
not in the list.

The list is in the Thread Stock window (**Save the List of Threads to
Buy** writes it as CSV), Design info shows the number of threads to buy,
and the production worksheet lists them.
