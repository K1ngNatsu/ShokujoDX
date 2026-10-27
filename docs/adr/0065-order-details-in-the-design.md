# ADR 0065: Order details in the design

Date: 2026-10-27 · Status: Accepted (owner request, P13 Milestone 2)

## Context

The person at the machine and the customer need the order with the
design: who it is for, the order number, the garment, the sizes, where the
design goes and when it is due. Before this change the sheets had only the
design notes, and the quote of ADR 0064 kept its order in the window, so
it was lost when the program stopped.

A shop management system keeps orders in a database, with many designs
for each order. P13 does not build such a system ("Not in this phase").

## Decision

1. **The order is part of the design** (`Design::order`, an
   `emb_model::order::OrderInfo`): customer, order number, garment and its
   color, sizes with a count each (or one quantity), placement, due date
   (text as typed; the window warns if it is not `YYYY-MM-DD`), the name of
   the price list, new design, rush, garment price and notes for the
   order. A design with no order has `None`, and the project file has no
   `order` field, so older programs read it as before; unknown fields of a
   later program are ignored.
2. **One design, one order at a time.** A shop that sews a design for a
   second order saves a copy of the design, or changes the order. A
   template keeps no order, and a new design from a template starts with
   none.
3. **Edits are undo steps.** The Order Details window and the order fields
   of the Quote window change the design through `Document::edit_merged`:
   typing in one field is one step. An empty order removes the order.
   Drawing either window changes nothing.
4. **The quote uses the order.** The quantity is the sum of the sizes, or
   the quantity when there are no sizes. The price list is found by its
   name (the first list if the name is gone or empty).
5. **Where the order shows.** The production worksheet lists the order
   rows first; the approval sheet lists them with the price of the quote
   (when the list has prices); the quote sheet lists them; the order notes
   come after the design notes. The worksheet barcode has the order
   number in place of the file name. The design properties CSV gets six
   columns at the end (order, customer, garment, quantity, placement,
   due). The output profile can put the order number before the file
   names. Design info shows the order number and customer.

## Consequences

- Sheets stay in English (as before); the order values are the user's text.
- The placement is text, with a menu of the places of the products of
  P7 Milestone 5; the sheets do not draw the product picture.
- A database of orders, if a later phase needs one, can read the orders
  from the project files or the CSV.
