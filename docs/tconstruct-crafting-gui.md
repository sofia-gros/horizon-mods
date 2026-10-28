# M4 制作・組立の対象範囲

固定原作 `f4e77d43f39ba1047bf93ae816b7e1beaab514c2` の wood / flint pickaxe 部分集合。Part Builder に再利用する pattern と材料を入れ、head / handle / binding を選ぶ。Station に三部品を入れてピッケルを組み立て、同じ Station で既存の wood / flint 修理も行える。Pattern と二つのテーブルの形状は原作 `recipes/tables/{pattern,part_builder,tinker_station}.json` を基準とし、木材 tag に限定する。両テーブルには原作 texture を使用する。

GUI、スロット、選択、処理計画は公開された汎用基盤を使い、素材とレシピは Mod 側で定義する。機械 fixture は入力、永続収納、複数出力、ボタンイベントと進捗同期を組み合わせる。永続収納 fixture は可変数のスロットを使う。ホストの現行上限は入力36、出力8、収納50で、収納への hopper 自動化は対象外。保存済み収納のスロット ID と順序の変更には移行が必要。複数出力は一度の操作で取得し、残りの出力を入れる持ち物の空きを事前確認する。

これは tinco 全体の完成を意味しない。他の素材・道具、動的外観、溶解・鋳造などは残る。専用サーバーでの検証は含まない。

分離済み Linux 実ゲームの環境準備は [実ゲーム検証](linux-tconstruct-game-verification.md) を参照。`tools/probe_tconstruct_crafting_gui.py` は選択、材料不足、中断・再開時の入力保持、部品3種、組立、保存復帰、採掘時の耐久消費を確認する。`tools/probe_tconstruct_station_repair.py` は既存修理を確認する。`tools/probe_generic_machine_gui.py`、`tools/probe_generic_storage_gui.py` と `tools/probe_generic_storage_players.py` は汎用 fixture を確認する。

## 原典GUIとの対応と差

Part BuilderとTinker Stationの背景は、固定原典 `f4e77d43f39ba1047bf93ae816b7e1beaab514c2` のMIT textureを使用する。Part Builderは原典のpattern・材料・結果位置、pattern選択の18ピクセル格子と通常・hover・選択画像を使い、player inventoryは両画面ともY=102に置く。Stationの結果位置は原典の `(114, 38)`。

Part Builderの選択肢は既存のhead・handle・bindingの3種類で、原典patternアイコンと選択状態を使う。16件以下なので原典どおりスクロールは無効。情報パネルには選択中の部品・必要数と投入材料の既存wood/flintデータを表示する。原典の全材料と詳細なtrait説明・tooltip・レシピ一覧は未対応。Stationは組立と修理の選択で入力を移動し、組立結果のみ名前を付けられる。既存ピッケルの単一部品交換とwood/flint修理に対応する。側面パネルは対応範囲を表示する簡易版で、原典の大型ツール表示、装備slot、詳細な材料・修飾子表示は未対応。組立時の未使用tool slotは画面内に残る。選択ボタンの位置も原典と異なる。素材・部品と修理は本Modの既存部分集合に限る。

Part Builderのpattern・材料slotは汎用storageを使用し、画面を閉じても内容が残り、ワールド保存・再読込後も復元される。選択は同一ブロックの画面を閉じて再度開いても共有される。ワールド再読込後の選択復元は行わない。情報panelは対応済みwood／flintの基礎値とtrait名に限定され、原典の詳細tooltipやtextのスクロールは未対応。Stationの四つの入力も収納として保存される。名前は画面ごとの未確定値であり画面を閉じると消える。
