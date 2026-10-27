# Tinkers’ Construct for Horizon

現在の開発ブランチでは、server-tools の同期 hook により基本 pickaxe の採掘性能とコア耐久を反映します。見た目は木のピッケルで、通常の耐久バーはコア状態を表示しません。[実ゲーム検証と画像](../docs/tconstruct-m3b4-verification.md) を参照してください。以下には過去の各段階の手順も残しています。

M0〜M2を検証済み。M3aでは性能を表示する開発用client Modに、状態の保存・通信を試すvanillaアイテム生成を追加しています。TConstructの性能を持つツール・作業台・溶融炉はまだ追加しません。

- 本体: `A:\Project\horizon`（`sofia-gros/horizon` フォーク）
- Mod ID / namespace: `tconstruct`、version 0.1.0
- Cargo package: `tconstruct-horizon`
- SDK: `sofia-gros/horizon` の `88b7bfafc01c7c475548d4379845ff4a9f944dc0` に固定し、Cargo.lockを管理。CLIはローカルの本体ブランチからビルド
- ゲーム: M1はインストール済みv0.2.66のin-frame。M2以降は各開発ブランチの本体と26.3準備済みresourceを使用
- manifest: API 0.1、client、`command` capability（既存API、プレイヤーの権限でvanillaコマンドを実行）。実験APIなし
- 対象原作: Minecraft 1.20.1系列のTinkers’ Construct 3
- 初期目標: 少数素材から部品・つるはしを作り、採掘・破損・修理する

[詳細実装調査](../docs/tconstruct-implementation-plan.md) に、本体側の追加箇所、Mod構成、API契約案、作業順序、検証条件をまとめています。
[先行調査](../docs/porting-tinkers-construct.md) に、固定した原作コミット、依存、MITライセンス、機能構成を記録しています。

フォルダ名はユーザー指定の `Tinkers' onstruct` です。ModのIDや自動生成されるpackage名とは分離して扱います。
このフォルダでPowerShellコマンドを実行する場合は、たとえば次のようにパスを引用します。

```powershell
Set-Location -LiteralPath "A:\Project\horizon-mods\Tinkers' onstruct"
```

## ゲームを起動して検証する

この端末にはビルド済み `target/horizon-mod/tconstruct.hmod` があります。

```powershell
Set-Location -LiteralPath "A:\Project\horizon-mods\Tinkers' onstruct"
& .\tools\play-diagnostics.ps1
```

GUIのゲームを起動します。v0.2.66とそのresourceを使い、`.local/manual-M1-<固有ID>/` に専用ワールド・設定・ログを作ります。音声・自動更新は無効です。終了後は起動元シェルの環境変数を戻します。毎回新しいワールドを作るので、以前の手動試験ワールドを開き直す手順ではありません。

ワールドが表示されたら **T** でチャットを開き、順に入力してください。引数はhead / handle / bindingです。

| 操作 | 期待結果（末尾に `; diagnostics only`） |
| --- | --- |
| `/tconstruct stats wood wood wood` | `Pickaxe: durability=60, mining=2.000, damage=1.000, speed=1.200` |
| `/tconstruct stats flint wood wood` | `Pickaxe: durability=85, mining=3.500, damage=2.250, speed=1.200` |
| `/tconstruct stats flint flint flint` | `Pickaxe: durability=72, mining=3.500, damage=2.475, speed=1.200` |
| `/tconstruct stats stone wood wood` | エラー `unsupported material tconstruct:stone` |
| `/tconstruct support` | コアの対応範囲と、未実装のゲームhook・ツール・機械を表示 |

終了はEscメニューから行います。再度スクリプトを実行すると、別の新規ワールドでも同じ診断を試せます。失敗時は表示された専用フォルダの `game.log` / `game-error.log` を確認してください。

パス検証だけなら `-ValidateOnly`、別のゲーム・resourceを指定するなら `-Client PATH -Resources PATH`。別版との互換性は未保証です。PowerShellの実行ポリシーを自動変更する処理は入れていません。

**制限:** v0.2.66 / v0.2.68の自動試験では、統合サーバーモードの `chat /tconstruct ...` がclient Mod判定を迂回し、診断コマンドが認識されませんでした。手動スクリプトは成功を確認したv0.2.66の `--in-frame` を明示します。M2ブランチでは操作経路の修正を検証中です（[M2検証](../docs/tconstruct-m2-verification.md)）。`horizon-ctl command` は統合サーバーconsole経路であり、client Modの操作経路とは区別します。

## ビルド・試験

Rustと `wasm32-unknown-unknown` が必要です。各コマンドの成功を確認してから次へ進みます。

```powershell
Set-Location -LiteralPath 'A:\Project\horizon'
cargo build --locked -p cargo-horizon-mod
rustup target add wasm32-unknown-unknown
Set-Location -LiteralPath "A:\Project\horizon-mods\Tinkers' onstruct"
cargo test --locked
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo build --locked --release --target wasm32-unknown-unknown
& 'A:\Project\horizon\target\debug\cargo-horizon-mod.exe' horizon-mod build
& 'A:\Project\horizon\target\debug\cargo-horizon-mod.exe' horizon-mod test
& 'A:\Project\horizon\target\debug\cargo-horizon-mod.exe' horizon-mod check target/horizon-mod/tconstruct.hmod --json
```

CLIをPATHにインストールしている場合は `cargo horizon-mod build / test / check` でも同じ処理です。成果物はGit管理しません。

## M0の入力検証・数値oracle

原作TConstruct `f4e77d43f39ba1047bf93ae816b7e1beaab514c2` の31ファイルを選択して取り込みました。MIT本文・著作権表示は `LICENSE` と `third_party/README.md`、出典・SHA-256は `upstream-lock.json` にあります。

```powershell
& 'A:\miniconda3\python.exe' -B tools/import_upstream.py --check
& 'A:\miniconda3\python.exe' -B tools/report_dependencies.py --check
& 'A:\miniconda3\python.exe' -B tools/generate_numeric_oracle.py --check --java 'C:\Users\metal\AppData\Roaming\PrismLauncher\java\java-runtime-epsilon\bin\java.exe'
```

Python・JDKはこの端末で確認済みの実行ファイルです。別環境では置き換えてください。Java oracleは選択した原作FloatToolStatメソッドを最小hostで実行して23ケースのfloatビット列を照合します。Forge・Mantle全体を起動する比較ではありません。

再取り込みはローカルcloneを指定して `tools/import_upstream.py --source PATH`。固定コミットのGitオブジェクトを読み、改変されたsnapshotは上書きしません。M0は入力・出典を確認する段階で、単独ではゲームへ機能を追加しません。

## 対応範囲と次の段階

コアはwood / flintのpickaxe、部品stats、cultivated修理係数、jagged条件式、pierceの数値補正・効果定義、耐久・broken・修理ポイント・JSON往復に対応します。修理ポイント操作はレシピの適用処理ではありません。効果のゲームへの付与、ランダム初期素材、upgrade、AOE、レシピ実行、menu、表示、fluid・機械は未実装です。

`port-support.json` に対応範囲・依存・未対応一覧を記録します。固定した未対応moduleは検証して保持しますが、実行済みとは扱いません。未知のmodule・素材・状態schemaはエラーにします。

段階ごとの状態は [ロードマップ](../docs/tconstruct-roadmap.md)、M1の結果は [M0・M1検証記録](../docs/tconstruct-m1-verification.md)、本体の更新は [上流追従記録](../docs/horizon-upstream-sync.md) を参照してください。

M2のserver接続は本体・Modとも `codex/server-mod-game-integration` ブランチで実装し、5構成53項目を検証しました。ゲーム起動は `& .\tools\play-server-probe.ps1`。実験API 0.3 / API level 1のprobeを使い、通常の統合サーバーとM1 client診断を同時に動かします。必要な本体・成果物・操作・期待結果・in-frameのNether制限は [M2検証記録](../docs/tconstruct-m2-verification.md) に記載しています。

## M3a: 状態付き試験アイテム

開発ブランチは本体・Modとも `codex/tconstruct-item-hooks`。`& .\tools\play-item-state.ps1` で分離したテストワールドを開き、次を入力します。終了後、`& .\tools\play-item-state.ps1 -ResumeLatest` で同じワールドを再開できます。

```text
/tconstruct specimen wood wood wood
/tconstruct specimen flint wood wood 17
/tconstruct specimen flint wood wood 85
```

`TConstruct state specimen` という名前の木のつるはしが合計3本入ります。内部の `minecraft:custom_data.tconstruct.state_json` にschema・tool・3素材・damage・brokenを格納します。最後の個体はコアの計算上damage=85 / broken=trueです。**ゲーム内性能・耐久・破損時の挙動はすべてvanillaの木のつるはしのまま**です。この段階のbrokenフラグは保存用fixtureであり、採掘を禁止するhookではありません。

`/give` を実行できる権限が必要です。通知は処理のキュー登録を示し、権限不足などでvanillaコマンドが失敗した場合はチャットのエラーも確認してください。未知素材は生成前に拒否します。`/tconstruct specimen stone wood wood` はエラーになりアイテムを増やしません。

自動試験はリポジトリルートの `tools/probe_horizon_item_state.py`。保存・再起動前後のinventoryを記録し、vanillaアイテムと3個体の素材・damage・brokenを比較します。ゲームを一つずつ起動します。詳細は [M3a検証記録](../docs/tconstruct-m3a-verification.md) を参照してください。

## M3b1: サーバーからの診断

補助server packageは [server-tools](server-tools/README.md)。API 0.3 / このforkのlevel 9の `items-read` / `items-edit` / `items-tools` を使い、SDKを `50c7c14650c8f05a245f93f81463da095ef9882c` に固定します。clientのSDK固定先は従来の `88b7bfaf` です。2つのcomponentは別々にビルドします。共通コアだけ使う場合はCargoの `default-features = false` でclient adapterのexportを除外できます。

`& .\tools\play-item-state.ps1 -Inspect` で両packageを読み込みます。3本を作って持ち替え、その都度 `/tconstruct_tool inspect` を入力すると、サーバーから読んだparts・damage・broken・コア基礎statsを表示します。`hooks=false` の診断段階で、ゲーム性能・耐久処理はvanillaのままです。保存後の再開は `-Inspect -ResumeLatest`。[M3bの期待値と検証範囲](../docs/tconstruct-m3b-verification.md) を参照してください。

M3b2ではCreative限定の `/tconstruct_tool test-damage 100` / `test-repair 20` で、選択個体のコアdamage・brokenを変更できます。Survivalでの実際の耐久消費・修理レシピは次の段階です。

M3b3では本体のTool componentをclient予測とserver採掘へ反映し、Creativeの `/tconstruct_tool activate` でコアstatsを個体に適用します。採掘によるコア耐久消費と攻撃性能は未実装です。実機手順・自動比較・未対応範囲は [M3b3検証記録](../docs/tconstruct-m3b3-verification.md) にあります。
