# 部品制作のコア

固定原作 `f4e77d43f39ba1047bf93ae816b7e1beaab514c2` の wood / flint 1:1 材料レシピと各 Part Builder レシピを読み、pick head 2 個分・handle 1 個分・binding 1 個分の材料数を算出します。pattern 条件は呼び出し側が検証済みであることを要求し、組み立てでは head / handle / binding の順を確認して fresh pickaxe の ToolState を作ります。

`IMaterialValue.getItemsUsed` の原作 Java メソッドを使う oracle と、Rust の不足材料・不正 pattern・未知素材・誤った部品順の試験を実施。原作の剰余物、1:1 以外の材料比率、素材 variant、全レシピは対象外です。

**このコードは純粋な計画のみで、実ゲームで材料を消費したり部品を作ったりしません。** Host に revision と全 slot の比較、材料消費と出力を一体で確定する汎用 transaction、Station の menu と保存を実装した後に接続します。プレビュー結果だけを信用して inventory を更新すると、同時操作による増殖・消失の危険があります。
