# M4 制作・組立の対象範囲

固定原作 `f4e77d43f39ba1047bf93ae816b7e1beaab514c2` の wood / flint pickaxe 部分集合。Part Builder に再利用する pattern と材料を入れ、head / handle / binding を選ぶ。Station に三部品を入れてピッケルを組み立て、同じ Station で既存の wood / flint 修理も行える。Pattern と二つのテーブルの形状は原作 `recipes/tables/{pattern,part_builder,tinker_station}.json` を基準とし、木材 tag に限定する。テーブルの原作 texture 継承は未対応。

GUI、スロット、選択、処理計画は公開された汎用基盤を使い、素材とレシピは Mod 側で定義する。機械 fixture は入力、永続収納、複数出力、ボタンイベントと進捗同期を組み合わせる。永続収納 fixture は可変数のスロットを使う。ホストの現行上限は入力36、出力8、収納50で、収納への hopper 自動化は対象外。保存済み収納のスロット ID と順序の変更には移行が必要。複数出力は一度の操作で取得し、残りの出力を入れる持ち物の空きを事前確認する。

これは tinco 全体の完成を意味しない。他の素材・道具、動的外観、溶解・鋳造などは残る。専用サーバーでの検証は含まない。

分離済み Linux 実ゲームの環境準備は [実ゲーム検証](linux-tconstruct-game-verification.md) を参照。`tools/probe_tconstruct_crafting_gui.py` は選択、材料不足、中断返却、部品3種、組立、保存復帰、採掘時の耐久消費を確認する。`tools/probe_tconstruct_station_repair.py` は既存修理を確認する。`tools/probe_generic_machine_gui.py`、`tools/probe_generic_storage_gui.py` と `tools/probe_generic_storage_players.py` は汎用 fixture を確認する。
