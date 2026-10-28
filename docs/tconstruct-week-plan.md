# TConstruct 移植の残作業と順序（2026-10-28）

対象は原作 TConstruct 1.20.1 の `f4e77d43f39ba1047bf93ae816b7e1beaab514c2`。現在の実装対象は wood / flint pickaxe の部分集合であり、全移植の完了範囲ではない。M0–M2 はロードマップ記載の条件で完了、M3 は基本採掘・近接と耐久の接続まで、M4 はレシピ計画と単一 inventory のメモリ上の transaction まで。M5–M8 は未完了。詳細と検証の制限は `tconstruct-roadmap.md` と `horizon-upstream-sync.md` を参照。

優先順位と依存関係（11月4日頃を目標とするが、完了の保証ではない）:

1. 10/28–29: M4 の汎用 block entity・menu・サーバー側 transaction、Part Builder / Station の制作・修理・交換。slot revision、同時操作、shift-click、切断・破壊、保存を扱う。純粋計算を先行しゲーム接続を独立して検証する。
2. 10/29–30: M5 の汎用 layer / tint / selector と素材・破損状態の表示。inventory、手持ち、drop と同期を検証。M4 と並行可能だが実ツールのデータ同期に依存する。
3. 10/30–11/1: M6 の fluid / tank、tick と保存・同期を汎用契約で追加し、Melter、Faucet、Casting Table の収支と冷却を実装する。
4. 11/1–2: M7 の bounded multiblock、Smeltery と合金。M6 の流体・機械基盤に依存する。
5. 11/2–4: M3 の残る modifier と M8 の全ツール・素材・modifier・防具・worldgen・book 等を原典と照合し、対応表、回帰、配布再現性を整備する。独立レビューの修正を各機能に随時反映する。

M8 の全データ・外部 Mod 連携・原作ワールド変換などの完全な受け入れ範囲は未定義で、実装量と検証環境の制約から一週間以内に完了できるか不明。除外を決定したものではない。互換性の対象、全原作データの棚卸し、Forge 実機との比較環境は引き続き確定が必要。

部品交換の純粋計画は `ToolStack.setMaterials` / `setStats` の性能再構築と耐久クランプを部分的に表す。現在の `ToolState` は破損後に最大耐久を増やしても破損フラグが残る原典の状態を表せないため、そのケースを拒否する。交換レシピの適合条件・実際の消費・UI・ゲーム内挙動は未実装であり、原作全体との一致は未検証。
