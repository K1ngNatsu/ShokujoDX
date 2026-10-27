---
id: quotes
title: Orders, quotes and run times
group: production
keywords: order details, customer, order number, sizes, placement, due date, barcode, quote, price, price list, cost, estimate, order, quantity, stitches, 1000 stitches, setup, digitizing, rush, tax, garment, heads, multi-head, hooping, run time, production time, pieces an hour
---
# Orders, quotes and run times

**Design > Order Details** keeps the order of the design: the customer,
the order number, the garment, the sizes, the placement and the due date.
**Design > Quote** gives the price of the order from your price lists, and
the time to sew it on your machine. It also makes a quote sheet for the
customer.

## Order details

- **Order number**, **Customer**, **Garment** and **Garment color**.
- **Placement**: type it, or choose a place from the menu (the places of
  the products, Design > Product).
- **Due date**: as `YYYY-MM-DD`. **Today** puts the date of today.
- **Sizes**: **Add a Size** for each size and its pieces, or **Add Usual
  Sizes** (XS to XXL). With no sizes, set the **Quantity**.
- **Notes for this order**: they come after the design notes on the sheets.

The order is saved with the design, and Undo takes back a change. A
template keeps no order: a new design from a template starts with none.
**Clear Order** removes the order of the design.

Where the order shows:

- The production worksheet starts its list with the order. Its barcode
  has the order number (else the file name).
- The approval sheet has the order and, when the price list has prices,
  the price.
- The quote sheet has the order.
- The design properties CSV has the columns order, customer, garment,
  quantity, placement and due.
- With **Order number before the file names** (Output design settings),
  the files are named, for example, `A-1042 rose.dst`.
- Design info shows the order number and the customer.

## Price lists

Open **Prices** in the Quote window and set the prices of the list. A
price of 0 adds nothing.

- **Each 1000 stitches**: the price of 1000 stitches of one piece.
- **Lowest price for each piece**: a small design costs at least this
  much for each piece.
- **Colors in the price** and **Each added color**: the stitch price
  includes some threads; each thread over that number costs more, for
  each piece. A thread that sews two times is one thread.
- **Setup**: once for each order.
- **Digitizing** and **Digitizing, each 1000 stitches**: once, for a new
  design only.
- **Quantity prices**: from a quantity, a percent off the embroidery
  price. The largest quantity that the order reaches applies.
- **Rush order**: a percent added to the sum of a rush order.
- **Tax**: a percent of the sum with the rush price.
- **Currency**: the text before the numbers, for example `$` or `€`.
  The thread cost in Options uses the same currency.

Click **New List** to make a copy of the list, for example one list for
shirts and one for caps. **Remove List** removes the list that shows.

## The order

Set the **Quantity** (with sizes, the sum of the sizes of Order Details),
the **Garment price** (if you supply the garments; 0 if the customer
supplies them), **New design** (adds the digitizing price) and **Rush
order**. They are part of the order of the design, and the price list
that you choose is kept with it. The window shows each line of the price,
the total and the price of each piece. Each line is in cents, so the lines
add up to the total.

The thread cost of each piece (from Options) shows under the price. It is
for you: it is not on the quote sheet.

## Run time

The run time is the time to sew the whole order. It uses the times of
**Design > Machine Settings**:

- **Heads**: the pieces that the machine sews at a time. One cycle sews
  one piece on each head.
- **Hoop one piece**: the time to hoop one piece and to take it out of
  the hoop after it sews.
- **Frame change**: the time to take the hoops off the machine and put
  the next hoops on.
- **Hoop the next pieces while the machine sews**: you have a second set
  of hoops. Then a cycle takes the longer of the sew time and the time
  to hoop the pieces of all heads. Without it the times add.

The sew time of one piece is the sew time of Design info, with the color
changes, trims, stops and thread changes. The window says if the machine
or the operator sets the speed. If the operator sets the speed, a second
person to hoop the pieces makes the order faster.

Design info shows the price of each piece and the run time of the
quantity in the Quote window.

## Quote sheet

**Save Quote Sheet** writes a PDF page with the design picture, the
lines of the price, the total and lines to sign. The paper size is the
paper of the output settings.

## From the command line

`emb-cli quote design.embproj --prices prices.toml --quantity 48 --machine tajima --heads 6`
prints the same lines and the run time. With no `--quantity`,
`--new-design`, `--rush` or `--garment`, the order of the design gives
them. The price list file has the
fields of a price list, for example:

```toml
name = "Shirts"
per_1000 = 1.50
min_piece = 5.00
setup = 20.00

[[breaks]]
from = 24
percent_off = 10
```

`--sheet quote.pdf` writes the quote sheet.
