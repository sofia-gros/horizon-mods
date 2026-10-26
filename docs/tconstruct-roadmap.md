# Tinkers’ Construct移植ロードマップ

開始日: 2026-10-26。対象配置・設計は [詳細実装調査](tconstruct-implementation-plan.md)、実機検証の土台は [検証自動化](verification-automation.md) を参照します。

最初の遊べる目標は、wood / flintの部品でpickaxeを組み立て、採掘・破損・修理・保存復帰できることです。その後に溶融・鋳造・multiblockと対応ツールを増やします。
各段階を実装・試験してから完了へ更新します。「原作と同じ」は実際に比較した範囲に限って使います。

| 段階 | 状態 | 実装するもの | 完了条件 |
| --- | --- | --- | --- |
| M0 再現可能な入力 | 完了 | 原作コミット固定、MIT・出典・SHA-256、選択データ、依存・対応範囲一覧 | 27ファイルと依存一覧の検証成功。未知素材・moduleを拒否 |
| M1 Modコアと診断Mod | 完了 | 素材・部品・stats・traitの小集合、ToolState、耐久・修理、Wasm診断コマンド | Rust・Java oracle・Wasm・sandbox test/check成功。v0.2.66 in-frameの診断と保存復帰確認 |
| M2 サーバー基盤 | 完了（検証範囲・制限は記録参照） | 上流API 0.3と未マージPR #1894を統合。ゲーム側lifecycle接続、dev packageのserver引き渡し、probeを追加 | 5構成53項目成功。旧client・dimension・pause・複数人の出力分離確認。in-frameのNetherは既存制限として除外 |
| M3 アイテム接続 | 作業中（M3a・M3b1・M3b2完了、性能hookは未実装） | custom data往復、個体性能、採掘・戦闘・耐久hook、broken状態 | toolが消失せず、他個体・vanillaを壊さず保存・同期できる |
| M4 制作・修理UI | 未着手 | generic Block Entity / menu / transaction、Part Builder・Station | 部品作成・組み立て・交換・修理。複数人・shift-click・保存で複製も消失もない |
| M5 動的な外観 | 未着手 | 素材別sprite / tint / tooltip、broken / modifier表示 | 手持ち・inventory・dropで表示が一致。固定視点の画像試験 |
| M6 最小溶融・鋳造 | 未着手 | fluid / tank、Melter・Faucet・Casting Table | 燃料・温度・液量・型・冷却・出力、保存復帰と描画を検証 |
| M7 Smeltery・合金 | 未着手 | bounded構造検証、multiblock、合金、chunk寿命 | 構造変更・unload・複数流体・同期・負荷を検証 |
| M8 対応拡大・公開準備 | 未着手 | tool / modifier / 素材 / worldgen / book / 防具等を順次追加 | 対応表・回帰試験・ライセンス・配布再現性が揃う |

## 現在の作業

M0とM1を実装・検証しました。本体の汎用API開発に原作固有コードを混ぜず、`Tinkers' onstruct/` にコアと診断Modを置いています。[検証記録](tconstruct-m1-verification.md) とMod READMEに直接検証手順と限界を記録しました。
2026-10-26、ユーザーによるM1実機検証でも、案内した期待値との一致が確認されました。
診断Modは既存APIで数値を観測するための足場で、採掘可能なTConstructツールの完成ではありません。
M2以降ではフォーク本体に変更を追加し、各段階を小さい差分へ分割します。
M2は本体・Modとも `codex/server-mod-game-integration` で実装。[起動・操作・期待結果と検証制限](tconstruct-m2-verification.md) を確認してください。通常の統合サーバーを手動検証の基準にします。

M2はユーザーによる実機検証でも期待どおりと確認済み。M3は本体・Modとも `codex/tconstruct-item-hooks` に分離しました。最初のM3aでは汎用custom-data送信修正と状態付きvanilla試験アイテムを扱います。M3bで必要になる個体性能・採掘・戦闘・耐久hookは現行WITに存在せず、試験アイテムでは動作しません。名前の変更や状態保存だけでM3全体を完了とは扱いません。
M3aはコア13件・sandbox 7ステップ・成果物check、統合サーバー / in-frameの実ゲーム12項目に成功。修正前の状態欠落も再現しました。[M3aの起動・操作・期待値と制限](tconstruct-m3a-verification.md) を参照してください。
2026-10-26、ユーザーによるM3a実機検証でも、保存・再起動後に試験アイテム3本が残ることを確認済み。
M3b1ではfork独自の実験capability `items-read` と補助server package `tconstruct_tools` を追加し、サーバーが読む個体状態と基礎statsを診断します。採掘・戦闘・耐久の変更は次の区切りです。[M3bの範囲・検証手順](tconstruct-m3b-verification.md)。
2026-10-27、M3b1の統合サーバー / in-frameで持ち替え・保存復帰・非対象品の拒否を計26項目、旧SDKのM2 server probeを14項目確認。いずれも成功。`hooks=false` の段階で、実機の人による確認は次の手動手順で行います。
2026-10-27、ユーザーからM3b1（本体 `235cac33` / Mod `0272914`）の実機確認済み・問題なしとの報告を受領。M3b2の比較付き状態更新・Creative限定テストdamage/repairは統合サーバー / in-frameの計60項目、旧SDKのM2回帰14項目を自動確認済みです。実際の採掘・戦闘による性能・耐久処理は引き続き未実装です。ユーザーの就寝中も自動検証して次の段階へ進める指示に従い、M3b2の手動確認を待たずM3b3に着手しています。

2026-10-26: 上流 `c9f554d1` を含むマージ `88b7bfaf` で実験的server Mod API 0.3のWIT・SDK・runtimeが追加され、M2で既存PR #1894のゲーム接続を専用ブランチへ統合しました。未マージPRの機能をupstream/mainの実装済み機能とは扱いません。[追従記録](horizon-upstream-sync.md) も参照してください。

## 検証の扱い

- 原作データ・選択したJava計算メソッドとの比較、Forge全体での比較、Horizon実機を区別して記録する。
- Modコアは `cargo test`、Wasmは `cargo horizon-mod test / check`、本体は該当crateと影響するparity試験を使う。
- 既存ワールドを使わず、runごとのhome・cwd・endpoint・ログへ分離する。
- 未実行の試験と未対応のhookはREADME・進捗に残し、機能があるように表示しない。
- 各段階の完了時に、ユーザー自身が再現できる本体/Modの版、起動コマンド、ゲーム内操作、期待結果を案内する。ゲーム接続前の段階は、非ゲームの確認手順と限界を明示する。
