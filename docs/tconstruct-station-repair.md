# Tinker Station の限定修理

対象は固定原作 1.20.1 `f4e77d43f39ba1047bf93ae816b7e1beaab514c2` と既存の wood / flint pickaxe コア。Tinker Station の tool 入力1枠・材料入力1枠・結果枠で、対応する head の火打石または plank 材料レシピを使って修理する。部品素材、damage、broken、採掘・攻撃 profile を更新する。結果はプレビューであり、取得時にのみ tool と必要数の材料を消費する。未実装の素材・repair kit・複数材料入力・他ツール・modifier は対象外と診断上区別し、完全移植から除外したことにはしない。

ブロック `tconstruct:tinker_station` は当面 vanilla smithing table の外観を使う。GUI は2入力と結果を持つ汎用 workstation の画面で、原作の Station の完全な外観・全操作ではない。登録された block の所有 content が指定した server Mod だけが recipe を提供する。入力が変わると古い preview は無効になり、サーバーは結果の取得前に block・距離・handler・入力と preview を再検査する。クライアントの表示はサーバーの結果を使う。Mod には `workstations` 実験権限を明示し、既存の `items-edit` の任意既存文字列編集契約は変更しない。

ゲームでは Creative で `/setblock ~ ~ ~2 tconstruct:tinker_station`、`/tconstruct specimen flint wood wood 85` で試験個体を用意し、火打石を用意する。Survival にして Station を右クリック、tool と火打石を2入力枠へ移す。結果枠を取ると修理済み tool と残りの材料が得られる。wood head では `minecraft:planks` のうち `tconstruct:wood_variants/planks` を除く材料を使う。試験用 carrier の生成は Creative 限定で、通常の tool 作成は M4 の残作業。

検証用 `tools/probe_tconstruct_station_repair.py` は run ごとに world と home を分離し、統合サーバーと `--in-frame` で flint / wood の結果プレビュー、取得後の収支と保存復帰を検査する。実行環境の詳細は [Linux 実ゲーム検証](linux-tconstruct-game-verification.md) を参照する。多人数同時操作、専用サーバー、全 UI 操作、原作 Forge との画面・挙動比較は別途必要。
