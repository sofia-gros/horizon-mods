# 標準耐久バーの実ゲーム画像

Horizon `1ab61541` の in-frame 実行画面。木のピッケルに見える flint / wood / wood の TConstruct 個体を Creative で有効化し、検証用 `test-damage` で段階的に消耗させました。通常の採掘で同じ耐久値が更新されることは別の自動試験で確認しています。

- [未消耗: 0 / 85](full.png)（バーは非表示）
- [消耗: 42 / 85](worn.png)（ホットバーに黄色いバー）
- [破損: 85 / 85](broken.png)（アイテムは残る）

チャットの `damage` は Mod の状態、`vanilla_damage` は標準 damage component です。最大値も 85 を設定しています。表示を見やすくするため耐久値は検証コマンドで設定しており、この画像だけで連続採掘を示すものではありません。
