# ピッケルの近接攻撃: 原作調査と着手範囲

原作 `f4e77d43f39ba1047bf93ae816b7e1beaab514c2` の `ToolAttackUtil` とアイテムタグを固定して確認。pickaxe は melee weapon だが melee primary ではないため、成功した生物への攻撃の基本耐久消費は 2、非生物は 0。broken は攻撃不可。Creative・modifiers・例外・攻撃対象と武器の特定はゲーム接続側で扱う必要があります。

コアの `damage_after_melee` は成功した非 Creative の攻撃を前提に、基本の耐久消費と broken 境界だけを実装します。Java の原作コード断片を使う `tools/generate_melee_wear_oracle.py` と Rust 単体テストで検証。**まだゲームの攻撃経路へ接続しておらず、攻撃力や攻撃速度、modifier と原作全体の互換性は未検証です。**

次の実装では、攻撃が成功した行為者の実際の個体と対象を同期的に扱い、手持ち変更や複数プレイヤーによる誤更新を避けます。本体の API は TConstruct 固有計算を含めず、別 Mod も使える契約とします。複数 Mod が同じ武器へ作用する場合の順序と衝突は別途設計が必要です。
