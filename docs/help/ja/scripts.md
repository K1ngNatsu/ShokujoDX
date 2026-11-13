---
id: scripts
title: スクリプト
group: reference
keywords: スクリプト, rhai, 自動化, マクロ, ツール, 一括, プラグイン, すべてのフィルに輪郭, サテンの密度, 糸の名前, 出力前の確認, emb-cli script
---
# スクリプト

スクリプトは、デザインを変える小さなツールです。たとえば各フィルの周りの輪郭や、すべてのサテンに同じ密度を付けるものです。**編集 > スクリプト**で、プログラムのサンプルスクリプトと自分のスクリプトを実行します。

## スクリプトを実行する

1. オブジェクトを選択します(何も選択しない場合、スクリプトはデザイン全体を受け取ります)。
2. **編集 > スクリプト**をクリックし、スクリプトを選びます。ウィンドウに、その働きと設定が表示されます。
3. 設定をして、**スクリプトを実行**をクリックします。

スクリプトのメッセージは**メッセージ**の下に表示されます。**元に戻す**で、スクリプトのすべての変更を 1 回で取り消します。スクリプトがエラーで止まった場合、デザインは変わりません。

スクリプトが変えられるのはデザインだけです。ファイルを開いたり保存したり、ネットワークを使ったり、プログラムを起動したりはできません。5 秒より長く動くスクリプトは止まります。

## 自分のスクリプト

スクリプトは、拡張子 `.rhai` のテキストファイルで、言語は Rhai です(rhai.rs を見てください)。設定フォルダーの `scripts` フォルダーに入れます: **スクリプトフォルダーを開く**でそれを開き、**マイスクリプトにコピー**で、変更用にサンプルをそこへコピーします。ファイルを変更したあとは、丸い矢印をクリックしてフォルダーを読み直します。

`//!` で始まる最初の行が、スクリプトを説明します:

```rhai
//! Outline Every Fill
//! Adds a running outline round each fill.
//! input distance: 0.0 "Distance from the edge (mm)"
```

最初の行がタイトル、次の行が説明で、各 `input` の行が 1 つの設定です: 名前、既定値(数値、`true` か `false`、または引用符で囲んだ文字)、ラベルです。スクリプトは値を `input.distance` として読みます。

## 関数

オブジェクトは番号(`id`)、糸も番号(`t`)です。サイズは mm です。

デザイン:

- `objects()`: 縫い順のすべてのオブジェクト。`selection()`: 選択したオブジェクト。`select([id, …])`: スクリプトの後の選択。
- `design_name()`、`set_design_name(text)`、`notes()`、`set_notes(text)`。
- `info()`: ステッチの数値: `stitches`、`colors`、`color_changes`、`trims`、`jumps`、`width_mm`、`height_mm`、`warnings`。

1 つのオブジェクト:

- `kind(id)`: `"fill"`、`"satin"`、`"running"`、`"applique"`、`"lettering"`、`"vector"`、`"stitches"`、`"cross_stitch"`、`"bling"`、または `"sketch"`。
- `name(id)`、`set_name(id, text)`、`thread(id)`、`set_thread(id, t)`。
- `size(id)`: `x`、`y`、`width`、`height`。
- `settings(id)`: ステッチの設定のマップで、プロジェクトファイルの名前を使います(たとえばフィルやサテンの `spacing_mm`)。`set_settings(id, map)` で設定します:

  ```rhai
  let s = settings(id);
  s.spacing_mm = 0.4;
  set_settings(id, s);
  ```

- `sew_index(id)`、`move_to(id, index)`: 縫い順の位置(0 が最初)。
- `remove(id)`。

複数のオブジェクト:

- `move_objects([id, …], dx, dy)`、`rotate_objects([id, …], degrees)`、`scale_objects([id, …], factor)`(オブジェクトの中心を基準に)。
- `add_outline([id, …], distance)`: ランニングの輪郭(距離が 0 より大きければ外側、小さければ内側)。`add_border([id, …], width)`: 外側のサテンのボーダー。どちらも新しいオブジェクトを返します。
- `shape([id, …], "weld")`: シェイピング(`weld`、`intersect`、`exclude`、`front_minus_back`、`back_minus_front`、`flatten`、`divide`、`combine`)。新しいオブジェクトを返します。

糸:

- `threads()`: `id`、`name`、`color`(`"#rrggbb"`)、`brand`、`code` を持つマップの一覧。
- `add_thread(name, "#rrggbb")`: 新しい糸。
- `set_thread_name(t, text)`、`set_thread_color(t, "#rrggbb")`、`set_thread_code(t, brand, code)`。

メッセージとエラー:

- `print(text)`: ウィンドウのメッセージ。
- `throw "text"`: エラーで止めます。デザインは変わりません。

## コマンドラインから

`emb-cli script my-tool.rhai design.embproj --out done` で、各プロジェクトファイルにスクリプトを実行し、変更したファイルを `done` フォルダーに保存します。`--set distance=1.5` で入力を設定します。`--out` がない場合は、メッセージを表示するだけで、何も保存しません。
