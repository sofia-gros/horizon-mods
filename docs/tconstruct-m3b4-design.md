# M3b4: 採掘による耐久消費の着手調査

状態: 調査中。以下は実装済みのAPIではありません。基準となる本体は `50c7c14650c8f05a245f93f81463da095ef9882c`、Modは `d00d53b2782e0bab1d1e7a19d1d3f84ccdae386b` です。既存の採掘プロファイルは耐久を消費しません。

Cloud移行時に、Modコアへ採掘後のpickaxe基本耐久計算を追加した。固定済みJavaの `getDamage` を抽出した4ケースと比較し、連続消費・破損境界・状態の保存往復も単体検証している。これはゲームへの接続ではない。Creative判定、破壊成功、正しい個体への同期適用、modifier処理は呼び出し側の契約が必要で、M3b4は未完了。

Linux Cloudでは本体 `50c7c146` とserver package `52a666a21628fe9a0d1d24bfcade52222dc5e2963fe43f96d9a78e1b85f4d4f6` を使い、統合サーバーとin-frameでM3b3の採掘プローブ各13項目を再確認した。実際の採掘ではまだコア耐久が変化しないことも確認している。上流 `43f53880` の型付きserver commandは既存 `server-events` のABIを変え、旧server packageの読み込みが `commands` の型不一致で失敗したため、未統合。互換性を保つ移行方法と旧package回帰の検証が必要。

## 観察

- 本体の `Game::player_destroy_block` は破壊に成功すると `mod_block_event` を記録し、その後に `mine_block_with_held` とドロップ処理を行います。現行のMod通知は `tick_server_mods` で後からまとめて配送されます。破壊の直後に持ち替えた場合、その時点で再取得した手持ちは破壊に使った個体とは限りません。
- 複数回の破壊を同じtick内にまとめると、tick開始時の同一snapshotを使った複数更新は競合します。破壊ごとに順序を保って適用する必要があります。
- 接続プレイヤーの操作は `as_guest` で一時的に `Game` のinventoryとguestのinventoryを交換します。交換中のguestレコードは行為者の最新inventoryではありません。既存の通常コマンド用snapshot・更新経路をそのまま流用すると、別プレイヤーの所持品を書き換える危険があります。
- SDKの既存 `server-events` のregistrationを変更すると、旧packageとの互換性に影響します。本体の `ui-host` は別worldのexportを任意で読み取る前例があり、新しい同期hookも同様に既存server exportを壊さず追加できるか検討します。
- 原作固定版のpickaxeは、採掘対象の硬度が0でなければ基本1ポイント、0なら0ポイントを使用します。原作modifierによる変換・特殊lootは別の対応範囲です。Creativeでの消費はありません。

## 実装前に確定すること

破壊成功とドロップ判定の順序、破壊時のアイテムsnapshot、ゲームモード・硬度、guest操作中のプレイヤーIDと実inventoryを契約として定義します。Modはコア状態と素材計算を所有し、本体はアイテム固有のJSONやTConstructの計算を解釈しません。例外や不正な更新では部分書き込みを取り消し、既存Modはそのまま動かします。

完了条件には通常とin-frameのゲーム両方式、1回と連続破壊、即時破壊、Creative、破損境界、持ち替え、2人の独立性、save/reopen、Mod例外、旧package互換を含めます。上流のMod関連変更を再fetchしてからAPIを設計・実装します。
