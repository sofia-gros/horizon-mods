# Windows でダブルクリックして試遊

**Windows 実機ではまだ検証していません。** 現在の Mod は未完成です。非 Unix 環境では、ランダムな余りが必要なリサイクルは取得できません。

1. [Windows 検証ブランチを ZIP でダウンロード](https://github.com/sofia-gros/horizon-mods/archive/refs/heads/windowns-all-usertest.zip)して展開します。既存の検証ブランチの checkout があれば、それを使っても構いません。
2. 展開したフォルダー内の **`Play-TConstruct.cmd`** をダブルクリックします。
3. 初回は本体のソース取得と正式 Mixins 変換・ビルドが実行されます。必要なツールや資源がない場合は画面に説明が出て停止します。Horizon 26.3 の資源を自動で見つけられなかったときだけ、準備済み資源フォルダーを選択してください。

初回に Git、Python 3.12 以降、Rust stable の MSVC toolchain、Visual Studio Build Tools の「C++ によるデスクトップ開発」、準備済み Horizon 26.3 リソースが必要です。これらのインストールや管理者昇格は自動では行いません。既存 PC 設定にあった Horizon Launcher の資源場所は自動検出対象です。別の場所でもフォルダー選択で指定できます。Vulkan 対応 GPU も必要です。

**コード上は Java と Forge の実行・インストールを必要としません。** Windows 実機では未確認です。Minecraft 側のレシピとタグは、既存の Horizon 26.3 資源から準備します。Minecraft 1.20.1 の jar は検索・取得しません。TConstruct の Forge 由来タグとレシピに必要な固定データは同梱されています。

初回の完了後は同じファイルを再度ダブルクリックすると、変更がなければビルドを再利用します。セーブと生成物は `%LOCALAPPDATA%\TConstruct-Horizon-Playtest` に分離され、`TConstruct-Playtest` ワールドで起動します。既存の Horizon や Minecraft のワールドは上書きしません。現在のビルド設定では音声・コントローラーは無効です。失敗した場合、ウィンドウは閉じず、原因と対処を表示します。中断したビルドは自動削除しません。

## 試せること

Part Builder を置いて Pattern と材料を入れ、候補を選んで部品を取得できます。木の板材から柄、calcite から石製 Helmet Plating を作る操作は Linux の検証用ビルドで確認しています。Minecraft アイテムのリサイクル耐久値は 26.3 の登録値を使うよう変更中で、正式変換版と Windows は未検証です。個別の最大耐久上書きや非 Minecraft アイテムは未対応です。金型や砂金型も一部操作できますが、自然な鋳造による入手、金型の詳細 tooltip、Station の全機能などは未完成です。取得の確認には Creative ワールドのチャットで、例えば以下を入力できます。

```text
/give @s tconstruct:part_builder 1
/give @s tconstruct:pattern 8
/give @s minecraft:oak_planks 16
/give @s minecraft:calcite 16
/give @s tconstruct:tool_handle_cast 1
/give @s tconstruct:blank_sand_cast 1
```

金型を Pattern 位置に入れると、金の Tool Handle Cast は部品取得時に残り、空の砂金型は成形時に消費されます。砂または赤砂 1 個を通常クラフトに置くと、それぞれ空の砂金型 4 個を作れます。Windows での画面と操作は未検証です。詳細な進捗は [README](../README.md) と [ロードマップ](roadmap.md) を参照してください。
