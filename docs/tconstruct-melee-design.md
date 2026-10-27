# ピッケルの近接攻撃: 原作調査と着手範囲

原作 `f4e77d43f39ba1047bf93ae816b7e1beaab514c2` の `ToolAttackUtil` とアイテムタグを固定して確認。pickaxe は melee weapon だが melee primary ではないため、成功した生物への攻撃の基本耐久消費は 2、非生物は 0。broken は攻撃不可。Creative・modifiers・例外・攻撃対象と武器の特定はゲーム接続側で扱う必要があります。

コアの `damage_after_melee` は成功した非 Creative の攻撃を前提に、基本の耐久消費と broken 境界だけを実装します。Java の原作コード断片を使う `tools/generate_melee_wear_oracle.py` と Rust 単体テストで検証。**攻撃力や攻撃速度、modifier と原作全体の互換性は未検証です。**

次の実装では、攻撃が成功した行為者の実際の個体と対象を同期的に扱い、手持ち変更や複数プレイヤーによる誤更新を避けます。本体の API は TConstruct 固有計算を含めず、別 Mod も使える契約とします。複数 Mod が同じ武器へ作用する場合の順序と衝突は別途設計が必要です。


## 直接 Mob 攻撃の基本耐久

本体 `dad7279efbcf2f34eb97c41afdce91a5a6138fff` で、所有 Mod へ成功した非 Creative の直接 Mob 攻撃を同期通知。server-tools が該当個体のコア状態、採掘速度、標準耐久を一体で更新します。両ゲーム方式で Creative の消費なし、成功時 2 消費、別個体の分離、破損限界、保存復帰を確認しました。失敗・trap 時の取消は本体 sandbox で検証済みです。

**攻撃力・攻撃速度の反映、破損状態での攻撃禁止、PvP、非生物への攻撃は未対応です。** 現時点では carrier の vanilla 攻撃力が使われます。攻撃の原作互換性は完成していません。
