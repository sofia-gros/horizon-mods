# 部品制作のコア

固定原作 `f4e77d43f39ba1047bf93ae816b7e1beaab514c2` の wood / flint 1:1 材料レシピと各 Part Builder レシピを読み、pick head 2 個分・handle 1 個分・binding 1 個分の材料数を算出します。pattern 条件は呼び出し側が検証済みであることを要求し、組み立てでは head / handle / binding の順を確認して fresh pickaxe の ToolState を作ります。

`IMaterialValue.getItemsUsed` の原作 Java メソッドを使う oracle と、Rust の不足材料・不正 pattern・未知素材・誤った部品順の試験を実施。原作の剰余物、1:1 以外の材料比率、素材 variant、全レシピは対象外です。

**このコードは純粋な計画のみで、実ゲームで材料を消費したり部品を作ったりしません。** Host に revision と全 slot の比較、材料消費と出力を一体で確定する汎用 transaction、Station の menu と保存を実装した後に接続します。プレビュー結果だけを信用して inventory を更新すると、同時操作による増殖・消失の危険があります。

## 材料修理の計画

木・火打石の 1:1 素材について、ピッケルの修理可能な head 素材と一致することを確認し、head の原作耐久値、素材レシピの 1/3 換算、`cultivated` の repair factor から消費数と回復後の ToolState を計画します。不足・不適合・未損傷は拒否し、入力状態自体は変更しません。固定原作の `MaterialRecipe.scaleRepair` と `TinkerStationRepairRecipe.repairFromSlot` の Java 断片による 4 ケースと照合しました。複数入力slotでは同一素材を全枠で検証し、枠順に端数を切り捨てて必要数を計画します。repair kit、その他素材・modifier は対象外です。

実際の材料消費や修理完了は実装されておらず、サーバーの認可・inventory transaction・Station UI との接続が必要です。

Horizon `81b3be3791171700c8105e5b0b218dfa434a12a1` の単一 inventory 向け比較・一括更新 primitive を、server-tools の開発用テストから固定参照しています。部品制作・組み立て・修理の計画を使い、同じ材料への二重要求、プレビュー後の部品変更、ツール状態の変更を模擬し、材料の誤消費や出力の重複を防ぐことを確認しました。これはメモリ上の統合試験であり、ゲームの inventory・ネットワーク・UI は未接続です。
