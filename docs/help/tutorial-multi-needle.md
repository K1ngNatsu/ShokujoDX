---
id: tutorial-multi-needle
title: "Tutorial: a multi-needle order"
group: tutorials
path: production
time: 20
level: practice
before: tutorial-output
keywords: tutorial, multi-needle, needles, needle sequence, thread stock, spools, threads to buy, order, order details, customer, order number, quantity, run time, heads, worksheet, operator, production
---
# Tutorial: a multi-needle order

In this tutorial you prepare an order of 120 polo shirts with a club crest
for a multi-needle machine: the needles, the threads that you have, the
order details, the run time and the worksheet for the operator.

## Open the start design

Click [Open the club crest](sample:club-crest). It has four colors: a navy
disc, a white star, a red chevron and a gold satin ring.

## The machine and its needles

1. On the **Design** tab, choose **Tajima multi-needle** in the
   **Machine** list. If the hoop of the design does not go on this
   machine, the program takes the smallest hoop of the machine that holds
   the design. The status line says which.
2. Click **Design > Machine and Hoop > Needles**. The **Needles** window
   shows the needle of each color in sew order.
3. Set **Needles on the machine** to the number of needles of your
   machine, for example 6.

![The Needles window](multi-needle-needles)

**Auto** puts a new thread on a free needle, and keeps a thread on the
needle that has it. To put a thread on a needle of your choice, choose the
needle in the list of its color. The **Needle sequence** is the order of
the needles: type it on the machine for a DST file. TBF and U01 files
have the needles in them.

When the design has more colors than the machine has needles, the machine
stops at each thread change (\*), and the operator puts the next thread on
the needle.

4. Close the **Needles** window.

After you sew the design, click **Keep these threads on the machine** in
the **Needles** window. The next design then uses the threads that are on
the needles where it can.

## Your threads

1. Click **Design > Orders > Thread Stock**.
2. Under **Add from a chart**, choose **Madeira Polyneon** in the
   **Thread chart** list.
3. Type `1967` in the field **name or code**, then click **Add** next to
   the thread (Light Navy). Each click adds one spool.
4. Add `1804`, `1637` and `1624` in the same way (Blanc de Blanc, Warm Red
   and Gold).
5. Under **Threads of this design**, each thread of the design shows the
   nearest thread in stock. Click **Use Threads in Stock**. The design now
   has your threads, with their codes.
6. Close the **Thread Stock** window.

## The order

1. Click **Design > Orders > Order Details**.
2. Type the **Order number** `A-1042`, the **Customer** `Riverside
   Football Club` and the **Garment** `Polo shirt`.
3. Set the **Quantity** to 120. Or click **Add Usual Sizes** and type the
   pieces of each size.
4. Close the **Order Details** window.

The order is saved with the design. The worksheet, the approval sheet and
the quote show it.

## The run time

1. Click **Design > Machine and Hoop > Machine Settings**.
2. Set **Heads** to 4: the machine sews four shirts at a time. Set the
   time to hoop one piece in **Hoop one piece**.
3. Close the **Machine Settings** window.

Design Info shows the run time of the order: the time to sew 120 shirts on
4 heads, with the time to hoop them.

## The threads to buy

Click **Design > Orders > Thread Stock** again. **Threads to buy for 120
pieces** compares the thread that the order needs with your spools. Each
thread on the list shows the spools to buy.

![The threads to buy](multi-needle-stock)

## The worksheet

Click **Design > Output > Print Preview**. The worksheet for the operator
has the order, the needle of each color, the threads that go on the
needles at the start, and the threads to buy.

![The worksheet with the needles](multi-needle-worksheet)

## If it does not look right

- **The Needles window asks for a machine.** Choose the machine of the
  design on the **Design** tab first.
- **Use Threads in Stock is gray.** Add threads to the stock first.
- **A thread changed to a color that is too different.** Undo, add a
  thread of a nearer color to the stock, and click **Use Threads in
  Stock** again. Under ΔE 2.3 the difference does not show.
- **The run time is not in Design Info.** Set a **Quantity** of more than
  one piece in **Order Details**.

## Next

- [Tutorial: a quote with a product picture](tutorial-quote)
- [Machines and hoops](machines-and-hoops): the needles of a multi-needle
  machine
- [Thread stock](thread-stock)
- [Orders, quotes and run times](quotes)
