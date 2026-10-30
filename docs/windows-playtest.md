# Windows でダブルクリックして試遊

**Windows 実機ではまだ検証していません。** 報告された起動エラーの原因調査は保留中です。現在の Mod は未完成です。ランダムな余りには Windows 対応の Rust 乱数を使いますが、Windows 実機では未検証です。

1. [Windows 検証ブランチを ZIP でダウンロード](https://github.com/sofia-gros/horizon-mods/archive/refs/heads/windows-all-usertest.zip)して展開します。既存の検証ブランチの checkout があれば、それを使っても構いません。
2. 展開したフォルダー内の **`Play-TConstruct.cmd`** をダブルクリックします。
3. 初回は本体のソース取得と正式 Mixins 変換・ビルドが実行されます。必要なツールや資源がない場合は画面に説明が出て停止します。Horizon 26.3 の資源を自動で見つけられなかったときだけ、準備済み資源フォルダーを選択してください。

初回に Git、Python 3.12 以降、Rust stable の MSVC toolchain、Visual Studio Build Tools の「C++ によるデスクトップ開発」、準備済み Horizon 26.3 リソースが必要です。これらのインストールや管理者昇格は自動では行いません。既存 PC 設定にあった Horizon Launcher と `A:\Project\horizon` の資源場所は自動検出対象です。別の場所でもフォルダー選択で指定できます。Vulkan 対応 GPU も必要です。

**コード上は Java と Forge の実行・インストールを必要としません。** Windows 実機では未確認です。Minecraft 側のレシピとタグは、既存の Horizon 26.3 資源から準備します。Minecraft 1.20.1 の jar は検索・取得しません。TConstruct の Forge 由来タグとレシピに必要な固定データは同梱されています。

初回の完了後は同じファイルを再度ダブルクリックすると、変更がなければビルドを再利用します。セーブと生成物は `%LOCALAPPDATA%\TConstruct-Horizon-Playtest` に分離され、`TConstruct-Playtest` ワールドで起動します。既存の Horizon や Minecraft のワールドは上書きしません。現在のビルド設定では音声・コントローラーは無効です。失敗した場合、ウィンドウは閉じず、原因と対処を表示します。中断したビルドは自動削除しません。

## 試せること

Part Builder を置いて Pattern と材料を入れ、候補を選んで部品を取得できます。木の板材から柄、calcite から石製 Helmet Plating を作る操作は Linux の検証用ビルドで確認しています。 4 種共通の原作説明文も追加し、英語の Helmet Plating を Linux 正式ビルドの実画面で確認しました。他の 3 種と Windows は未検証です。Minecraft アイテムのリサイクル耐久値は 26.3 の登録値を使います。Linux の正式 Mixins ビルドで、試験 datapack のメイスを使って消費・保存を確認しました。原作にメイスのリサイクルレシピはなく、Windows は未検証です。個別の最大耐久上書きや非 Minecraft アイテムは未対応です。金型や砂金型も一部操作できますが、Part Builder を開いている間は部品金型の材料コストを表示します。Linux 正式ビルドではレシピ変更後の表示更新を両実行方式で確認しました。リモート接続時の他画面での表示、自然な鋳造による入手、Station の全機能などは未完成です。Windows 実機は未検証です。取得の確認には Creative ワールドのチャットで、例えば以下を入力できます。

```text
/give @s tconstruct:part_builder 1
/give @s tconstruct:pattern 8
/give @s minecraft:oak_planks 16
/give @s minecraft:calcite 16
/give @s tconstruct:tool_handle_cast 1
/give @s tconstruct:blank_sand_cast 1
/give @s tconstruct:fake_storage_block[minecraft:custom_data={Material:"tconstruct:bronze"}] 1
/give @s tconstruct:pick_head[minecraft:custom_data={Material:"tconstruct:wood"}] 1
```

金型を Pattern 位置に入れると、金の Tool Handle Cast は部品取得時に残り、空の砂金型は成形時に消費されます。砂または赤砂 1 個を通常クラフトに置くと、それぞれ空の砂金型 4 個を作れます。Windows での画面と操作は未検証です。詳細な進捗は [README](../README.md) と [ロードマップ](roadmap.md) を参照してください。

素材付き収納ブロックの表示・保存・採掘を Linux で確認しています。Pick Block の素材引継ぎは両ゲーム方式のホストと LAN guest で、通常と Ctrl 付きの両方を確認しました。自然な入手と Windows 実機は未検証です。

部品の再利用は Linux の試験ビルドで、木の Pick Head 1個から木の Tool Handle 2個を作り、材料と Pattern の消費・保存再開を確認しています。正式 Mixins ビルドでも通常方式の同じ操作を確認しました。Windows 実機は未検証です。

結果スロットの右クリックと右 Shift-click は Linux の両方式で確認しました。隣接収納はチェスト・9枠収納・hopper の試作があり、hopper と dispenser の LAN 同期・同時取得を Linux で確認しました。Linux の LAN 試験でプレイヤー所持品から hopper への Shift 移動も確認しました。5枠表示とシュルカーへの入れ子拒否は Linux で代表確認しました。ダブルチェストは Linux の LAN 試験で両側のクリック・同期と結果取得の競合を確認しました。その他のスロット操作と詳細 callback は未完成です。Station ではフリント・木のピッケルを制作して採掘、修理、木の頭へ交換し保存再開する一連の操作を Linux の両方式で確認しました。LAN guest はコマンドで投入した部品から mattock のレイアウト選択・制作と基礎最大耐久 106 も確認しました。mattock の使用・修理・交換、他の工具・特性と Windows 実機は未検証です。

現在のランチャーは音声機能を含めずにビルドするため、ゲーム内の音は出ません。

対応金属の素材も共通のタグ・レシピから判定します。bronze を制作可能にする試験 datapack と Linux の正式 Mixins ビルドでは、bronze の Pick Head から柄を作れました。bronze の通常入手と Windows 実機は未検証です。

Part Builder の Minecraft 材料の容器返却は Horizon 26.3 のアイテム登録情報を使います。個別アイテムの component による上書きや独自の容器処理は未対応です。
銅塊は Horizon 26.3 にある Minecraft のアイテムを使います。bronze は銅とは別の素材です。
Minecraft に既にあるレシピや入手経路は維持し、足りない TConstruct 要素を追加する方針です。
