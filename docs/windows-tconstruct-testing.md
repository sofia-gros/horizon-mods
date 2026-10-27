# Windows 11: TConstruct の手動・自動テスト

Linux の Codex Cloud と別に、Windows 11 のログイン中のデスクトップでテストします。ゲーム・抽出リソース・ワールド・ログは各 PC の `.local` に置き、Git に追加しません。以下は PowerShell 7 を想定し、二つの checkout を隣接させます。既存 checkout がある場合はクローンせず、未コミットの変更を確認してからブランチを更新してください。

```powershell
$work = 'A:\Project'
New-Item -ItemType Directory -Force -Path $work | Out-Null
Set-Location -LiteralPath $work
git clone --branch codex/tconstruct-item-hooks https://github.com/sofia-gros/horizon.git
git clone --branch codex/tconstruct-item-hooks https://github.com/sofia-gros/horizon-mods.git
```

## 初回準備

想定する既存配置は `A:\Project\horizon`、`A:\Project\horizon-mods`、`C:\Users\metal\AppData\Roaming\PrismLauncher` です。これらが既に存在する場合は上の clone を実行せず、各 checkout の `git status` で変更を確認してから、指定ブランチを更新してください。

Git、Rust stable の MSVC toolchain、Visual Studio Build Tools の「C++ によるデスクトップ開発」、Python 3（`py -3`）をインストールし、新しい PowerShell を開いてください。ビルドには空き容量と時間が必要です。`-Jobs 1` で並列数を抑えられます。

Horizon Launcher は以下です。Prism Launcher は別のアプリとして扱います。

```powershell
& 'A:\Download\ABD\Compressed\horizon-launcher-windows-x64\horizon-launcher.exe'
```

Horizon Launcher の画面で対象バージョンを準備・起動してください。開発版の本体が読める準備済みリソースの場所を、後述の `$resources` に設定します。スクリプトは Prism のファイルや既存 server jar を自動選択しません。準備済みリソースは Launcher のバージョン配下の `.local\horizon-client\26.3` にあり、バージョンと開発版の互換性は実行して確認する必要があります。

```powershell
& 'A:\Project\horizon-mods\tools\setup-windows-tconstruct.ps1'
# 例。使用する Horizon Launcher のバージョンに合わせて変更してください。
$resources = Join-Path $env:APPDATA 'horizon-launcher\versions\v0.2.66\.local\horizon-client\26.3'
Test-Path -LiteralPath (Join-Path $resources 'horizon-resources.json')
```

準備スクリプトは Rust の Horizon、`horizon-ctl`、Mod ビルダー、両 Mod と共存試験用 probe をビルドします。`False` の場合は Launcher で準備したバージョンを確認し、`$resources` を変更してください。

## 手動で鉄鉱石と耐久を確認

```powershell
$work = 'A:\Project'
$play = Join-Path $work "horizon-mods\Tinkers' onstruct\tools\play-item-state.ps1"
& $play -Inspect -Resources $resources
```

新しい専用 Creative ワールドが開きます。T キーでチャットを開き、次を入力します。

```text
/tconstruct specimen flint wood wood 0
/tconstruct_tool activate
/tconstruct_tool inspect
```

生成された木のピッケルに見える個体を手に持って `activate` し、`inspect` の `damage=0`、`durability=85`、`hooks=mining` を確認します。Creative のまま `/setblock ~ ~ ~2 minecraft:iron_ore`、続いて `/gamemode survival` と入力し、鉄鉱石を採掘します。鉄の原石がドロップし、再度 `/tconstruct_tool inspect` すると `damage=1` です。通常の耐久バーにもコア耐久が反映されます。耐久値を見やすく変えるテストとして、Creative で `/tconstruct_tool test-damage 41` を使うと、damage 42 / 85 の黄色いバーを確認できます。二本目の個体を作り、持ち替えて採掘するとそれぞれの damage が別に増えます。Creative で採掘した場合と硬度 0 のブロックは消費しません。damage 85 で broken となり、さらに採掘してもツールは消えません。

終了後に同じワールドを再開するには `& $play -Inspect -Resources $resources -ResumeLatest`、in-frame 方式は `& $play -Inspect -Resources $resources -InFrame`。リソースや本体が別の場所なら `-Client 'C:\...\horizon-client.exe' -Resources 'C:\...\26.3'` を追加します。`-ValidateOnly` は起動前に入力ファイルを確認します。ログとワールドの場所はスクリプトが表示します。[Linux 実ゲームの前後画像](images/tconstruct-m3b4/README.md) も参照できます。

## 自動回帰

デスクトップのログイン中に実行します。専用ワールドとログを作り、統合サーバー・in-frame の採掘と保存復帰、二人接続、専用サーバー、別の server Mod との共存、Mob への近接攻撃を順に試します。複数のゲームが起動する場合があります。

```powershell
$test = 'A:\Project\horizon-mods\tools\test-windows-tconstruct.ps1'
& $test -Mode all -Resources $resources
# 個別実行: integrated / in-frame / players / dedicated / coexist / coexist-in-frame / melee / melee-in-frame
& $test -Mode integrated -Resources $resources
```

各試験の `RUN=...` に `report.json` と `game.log` が残ります。失敗時はその内容を確認してください。Windows 実機での新しい手順・自動試験の実行結果は未確認です。Cloud Linux での成功を Windows の成功とは扱いません。

Cloud Linux では PowerShell 7.5.4 でスクリプト構文、スペース入り checkout パスとテスト選択を確認し、PowerShell の自動試験入口から integrated の 9 項目を実行して成功しました。Linux 実行ファイルと仮想ディスプレイによる確認であり、Windows 向けバイナリや Windows の GUI 動作を確認した結果ではありません。
