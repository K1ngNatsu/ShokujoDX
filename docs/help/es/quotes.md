---
id: quotes
title: Pedidos, presupuestos y tiempos de producción
group: production
keywords: detalles del pedido, cliente, número de pedido, tallas, colocación, fecha de entrega, código de barras, presupuesto, precio, lista de precios, costo, estimación, pedido, cantidad, puntadas, 1000 puntadas, preparación, digitalización, urgente, impuesto, prenda, cabezales, varios cabezales, enmarcar, tiempo de producción, piezas por hora
---
# Pedidos, presupuestos y tiempos de producción

**Diseño > Detalles del pedido** conserva el pedido del diseño: el
cliente, el número de pedido, la prenda, las tallas, la colocación y la
fecha de entrega. **Diseño > Presupuesto** da el precio del pedido a
partir de sus listas de precios, y el tiempo para bordarlo en su máquina.
También crea una hoja de presupuesto para el cliente.

## Detalles del pedido

- **Número de pedido**, **Cliente**, **Prenda** y **Color de la prenda**.
- **Colocación**: escríbala, o elija un lugar del menú (los lugares de los
  productos, Diseño > Producto).
- **Fecha de entrega**: como `YYYY-MM-DD`. **Hoy** pone la fecha de hoy.
- **Tallas**: **Añadir una talla** para cada talla y sus piezas, o
  **Añadir las tallas habituales** (XS a XXL). Sin tallas, defina la
  **Cantidad**.
- **Notas de este pedido**: van después de las notas del diseño en las
  hojas.

El pedido se guarda con el diseño, y Deshacer devuelve un cambio. Una
plantilla no conserva ningún pedido: un diseño nuevo a partir de una
plantilla empieza sin pedido. **Borrar el pedido** quita el pedido del
diseño.

Dónde aparece el pedido:

- La hoja de trabajo de producción empieza su lista con el pedido. Su
  código de barras tiene el número de pedido (si no, el nombre de
  archivo).
- La hoja de aprobación tiene el pedido y, cuando la lista de precios
  tiene precios, el precio.
- La hoja de presupuesto tiene el pedido.
- El CSV de propiedades del diseño tiene las columnas order, customer,
  garment, quantity, placement y due.
- Con **Número de pedido delante de los nombres de archivo** (ajustes de
  Producir diseño), los archivos se llaman, por ejemplo,
  `A-1042 rose.dst`.
- La información del diseño muestra el número de pedido y el cliente.

## Listas de precios

Abra **Precios** en la ventana Presupuesto y defina los precios de la
lista. Un precio de 0 no añade nada.

- **Cada 1000 puntadas**: el precio de 1000 puntadas de una pieza.
- **Precio mínimo por pieza**: un diseño pequeño cuesta al menos esto por
  cada pieza.
- **Colores incluidos en el precio** y **Cada color adicional**: el precio
  de las puntadas incluye algunos hilos; cada hilo por encima de ese número
  cuesta más, por cada pieza. Un hilo que se borda dos veces es un hilo.
- **Preparación**: una vez por cada pedido.
- **Digitalización** y **Digitalización, cada 1000 puntadas**: una vez,
  solo para un diseño nuevo.
- **Precios por cantidad**: a partir de una cantidad, un porcentaje menos
  del precio del bordado. Se aplica la mayor cantidad que alcanza el
  pedido.
- **Pedido urgente**: un porcentaje que se añade a la suma de un pedido
  urgente.
- **Impuesto**: un porcentaje de la suma con el precio urgente.
- **Moneda**: el texto delante de los números, por ejemplo `$` o `€`. El
  costo del hilo en Opciones usa la misma moneda.

Haga clic en **Lista nueva** para hacer una copia de la lista, por
ejemplo una lista para camisas y otra para gorras. **Quitar la lista**
quita la lista que se muestra.

## El pedido

Defina la **Cantidad** (con tallas, la suma de las tallas de Detalles del
pedido), el **Precio de la prenda** (si usted pone las prendas; 0 si las
pone el cliente), **Diseño nuevo** (añade el precio de digitalización) y
**Pedido urgente**. Forman parte del pedido del diseño, y la lista de
precios que elija se conserva con él. La ventana muestra cada línea del
precio, el total y el precio de cada pieza. Cada línea está en céntimos,
así las líneas suman el total.

El costo del hilo de cada pieza (de Opciones) aparece debajo del precio.
Es para usted: no está en la hoja de presupuesto.

## Tiempo de producción

El tiempo de producción es el tiempo para bordar todo el pedido. Usa los
tiempos de **Diseño > Ajustes de la máquina**:

- **Cabezales**: las piezas que la máquina borda a la vez. Un ciclo borda
  una pieza en cada cabezal.
- **Enmarcar una pieza**: el tiempo para enmarcar una pieza y sacarla del
  bastidor después de bordarla.
- **Cambio de bastidores**: el tiempo para quitar los bastidores de la
  máquina y poner los siguientes.
- **Enmarcar las siguientes piezas mientras la máquina borda**: tiene un
  segundo juego de bastidores. Entonces un ciclo dura lo más largo entre el
  tiempo de bordado y el tiempo para enmarcar las piezas de todos los
  cabezales. Sin esta opción, los tiempos se suman.

El tiempo de bordado de una pieza es el tiempo de bordado de la
información del diseño, con los cambios de color, cortes, paradas y
cambios de hilo. La ventana dice si la velocidad la define la máquina o el
operario. Si la define el operario, una segunda persona que enmarque las
piezas hace el pedido más rápido.

La información del diseño muestra el precio de cada pieza y el tiempo de
producción de la cantidad de la ventana Presupuesto.

## Hoja de presupuesto

**Guardar la hoja de presupuesto** escribe una página PDF con la imagen
del diseño, las líneas del precio, el total y líneas para firmar. El
tamaño del papel es el papel de los ajustes de salida.

## Desde la línea de comandos

`emb-cli quote design.embproj --prices prices.toml --quantity 48 --machine tajima --heads 6`
imprime las mismas líneas y el tiempo de producción. Sin `--quantity`,
`--new-design`, `--rush` ni `--garment`, los da el pedido del diseño. El
archivo de la lista de precios tiene los campos de una lista de precios,
por ejemplo:

```toml
name = "Shirts"
per_1000 = 1.50
min_piece = 5.00
setup = 20.00

[[breaks]]
from = 24
percent_off = 10
```

`--sheet quote.pdf` escribe la hoja de presupuesto.
