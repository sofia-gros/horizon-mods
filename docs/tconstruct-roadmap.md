# Tinkers’ Construct移植ロードマップ

開始日: 2026-10-26。対象配置・設計は [詳細実装調査](tconstruct-implementation-plan.md)、実機検証の土台は [検証自動化](verification-automation.md) を参照します。

最初の遊べる目標は、wood / flintの部品でpickaxeを組み立て、採掘・破損・修理・保存復帰できることです。その後に溶融・鋳造・multiblockと対応ツールを増やします。
各段階を実装・試験してから完了へ更新します。「原作と同じ」は実際に比較した範囲に限って使います。

| 段階 | 状態 | 実装するもの | 完了条件 |
| --- | --- | --- | --- |
| M0 再現可能な入力 | 開発中 | 原作コミット固定、MIT・出典・SHA-256、選択データ、依存・対応範囲一覧 | 再取り込み・ハッシュ検証が可能。未対応が診断される |
| M1 Modコアと診断Mod | 開発中 | 素材・部品・stats・traitの小集合、ToolState、耐久・修理、Wasm診断コマンド | Rust試験、Java数値oracle、Wasm build、sandbox test/checkが通る。診断Modと実ツールを区別 |
| M2 サーバー基盤 | 未着手 | role別ABI・runtime・lifecycle probe、dev packageのserver引き渡し | 旧client互換、in-frame / integrated / dedicated、dimensionとpauseを確認 |
| M3 アイテム接続 | 未着手 | custom data往復、個体性能、採掘・戦闘・耐久hook、broken状態 | toolが消失せず、他個体・vanillaを壊さず保存・同期できる |
| M4 制作・修理UI | 未着手 | generic Block Entity / menu / transaction、Part Builder・Station | 部品作成・組み立て・交換・修理。複数人・shift-click・保存で複製も消失もない |
| M5 動的な外観 | 未着手 | 素材別sprite / tint / tooltip、broken / modifier表示 | 手持ち・inventory・dropで表示が一致。固定視点の画像試験 |
| M6 最小溶融・鋳造 | 未着手 | fluid / tank、Melter・Faucet・Casting Table | 燃料・温度・液量・型・冷却・出力、保存復帰と描画を検証 |
| M7 Smeltery・合金 | 未着手 | bounded構造検証、multiblock、合金、chunk寿命 | 構造変更・unload・複数流体・同期・負荷を検証 |
| M8 対応拡大・公開準備 | 未着手 | tool / modifier / 素材 / worldgen / book / 防具等を順次追加 | 対応表・回帰試験・ライセンス・配布再現性が揃う |

## 現在の作業

M0とM1を先に実装します。本体の汎用API開発に原作固有コードを混ぜず、`Tinkers' onstruct/` にコアと診断Modを置きます。
診断Modは既存APIで数値を観測するための足場で、採掘可能なTConstructツールの完成ではありません。
M2以降ではフォーク本体に変更を追加し、各段階を小さい差分へ分割します。

## 検証の扱い

- 原作データ・選択したJava計算メソッドとの比較、Forge全体での比較、Horizon実機を区別して記録する。
- Modコアは `cargo test`、Wasmは `cargo horizon-mod test / check`、本体は該当crateと影響するparity試験を使う。
- 既存ワールドを使わず、runごとのhome・cwd・endpoint・ログへ分離する。
- 未実行の試験と未対応のhookはREADME・進捗に残し、機能があるように表示しない。
