# M3b4 鉄鉱石採掘の実画面

2026-10-27のCloud Linux環境、in-frameモードで撮影した実ゲーム画面です。生成画像ではありません。開発中の同期採掘hookを使っています。

- [採掘前](iron-before.png): 木のピッケルを見た目のcarrierとし、部品が `flint / wood / wood` の状態。診断欄は `damage: 0`、最大耐久 `durability: 85`、採掘性能 `mining: 3.5` を示します。
- [採掘後](iron-after.png): 鉄鉱石を壊して鉄の原石が落ちています。診断欄の `damage: 1` は耐久を1消費したことを示します。自動操作の結果も `minecraft:raw_iron` が1個ドロップしたことを確認しました。

これは診断用のvanilla木ピッケルcarrierで、独自モデルや通常の耐久バーは未実装です。`vanilla_damage` はvanillaの別の値で、TConstructの消費値は `damage` です。原作のすべてのmodifier・特殊loot・修理レシピはまだ実装していません。
