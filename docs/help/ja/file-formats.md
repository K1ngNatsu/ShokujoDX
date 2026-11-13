---
id: file-formats
title: ファイル形式
group: reference
keywords: 形式, ファイル, 拡張子, 読み込み, 書き出し, 開く, インポート, エクスポート, dst, vp3, pes, pec, exp, jef, xxx, sew, hus, shv, tbf, tap, u01, dsb, dsz, 10o, dat, embproj, svg, pdf, ai, emf, wmf, png, jpeg, bmp, tiff, gif, gpl, csv, dxf, zip
---
# ファイル形式

## プログラムが書き出すミシンファイル

| 形式 | ミシン | 色 |
|---|---|---|
| DST | Tajima、ほとんどの多針ミシン | なし |
| VP3 | Husqvarna Viking、Pfaff | あり |
| PES | Brother、Babylock | あり |
| PEC | Brother(PES のミシン部分) | あり |
| EXP | Melco、Bernina | なし |
| JEF | Janome | あり |
| XXX | Singer | あり |
| TBF | Tajima、針と色付き | あり |
| TAP | Happy | なし |
| U01 | Barudan、低速機能付き | なし |

色のない形式では、**デザインを出力**で、ミシンファイルの隣に糸色ファイル(INF、EDR、COL)を書き出せます。TBF と U01 のファイルには各色の針が入ります(**デザイン > 針**を見てください)。DST、TAP、EXP では、ミシンに入力する針の順序が作業シートに書かれます。DST と TAP には糸切りのコードがありません: **デザイン > ミシンの設定**を見てください。

## プログラムが読むミシンファイル

**ファイル > 開く**と**ステッチを挿入**は、これらの形式をステッチのオブジェクトとして読みます:

DST、VP3、PES、PEC、EXP、JEF、XXX、SEW、HUS、SHV、TBF、TAP、DSB、DSZ、10O、100、U01、DAT、EXY、FXY、GT、STX、PHB、PHC、BRO、EMD、INB、JPX、KSM、MAX、MIT、NEW、PCD、PCQ、PCS、PCM、SPX、STC、ZHS、ZXY。

ミシンファイルにあるのは針落ち点で、形ではありません。密度を保ったまま拡大縮小するには、**アートワーク > 変換 > 輪郭を認識**を使います。

## ほかのファイル

| ファイル | 読む | 書く | 用途 |
|---|---|---|---|
| EMBPROJ | はい | はい | このプログラムのプロジェクト: すべてのオブジェクトと設定。 |
| SVG | はい | はい | ベクターのアートワークを読み込み、輪郭、デザイン画像、カッターファイルを書き出し。 |
| PDF | はい | はい | ベクターのアートワークを読み込み、作業シート、図案、テンプレートを書き出し。 |
| AI, EMF, WMF | はい | EMF | ベクターのアートワーク。 |
| PNG, JPEG, BMP, TIFF, GIF | はい | PNG | 背景の画像を読み込み、デザイン画像を書き出し。 |
| DXF | いいえ | はい | アップリケのカッターファイルとラインストーンのテンプレート。 |
| CSV | はい | はい | 糸チャートとチーム名を読み込み、デザインのプロパティとデザイン一覧を書き出し。 |
| GPL | はい | いいえ | 糸チャート(GIMP のパレット)。 |
| EMBFONT, BDF, FON | はい | EMBFONT | 刺しゅうフォントとアルファベット、ビットマップフォント。 |
| TTF, OTF | はい | いいえ | コンピューターの TrueType と OpenType のフォント。 |
| ZIP | はい | はい | アセットのパッケージ、メールで送信のファイル。 |

## ミシンと形式の名前

このプログラムのミシン、ファイル形式、糸のブランドの名前は、それぞれの所有者の商標です。プログラムがどのファイルを読み書きするかを示すだけです。メーカーはこのプログラムを試験も承認もしていません。新しいデザインは、まず端切れで試してください。
