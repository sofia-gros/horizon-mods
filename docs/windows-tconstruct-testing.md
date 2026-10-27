# Windows 11: TConstruct の手動・自動テスト

Linux の Codex Cloud と別に、Windows 11 のログイン中のデスクトップでテストします。ゲーム・抽出リソース・ワールド・ログは各 PC の `.local` に置き、Git に追加しません。以下は PowerShell 7 を想定し、二つの checkout を隣接させます。既存 checkout がある場合はクローンせず、未コミットの変更を確認してからブランチを更新してください。

```powershell
$work = Join-Path $HOME 'horizon-work'
New-Item -ItemType Directory -Force -Path $work | Out-Null
Set-Location -LiteralPath $work
git clone --branch codex/tconstruct-item-hooks https://github.com/sofia-gros/horizon.git
git clone --branch codex/tconstruct-item-hooks https://github.com/sofia-gros/horizon-mods.git
```

## 初回準備

Git、Rust stable の MSVC toolchain、Visual Studio Build Tools の「C++ によるデスクトップ開発」、Python 3（`py -3`）、JDK 25（`java` / `javac`）をインストールし、新しい PowerShell を開いてください。Rust は `rustup` からインストールできます。ビルドには数 GB 以上の空き容量と時間が必要です。`-Jobs 1` で並列数を抑えられます。

Minecraft Java Edition 26.3 は正規の Minecraft Launcher でインストールし、少なくとも一度起動して assets と libraries を揃えてください。別ディレクトリの正規インストールを使う場合は `-MinecraftDir` で指定します。Horizon の既存 `horizon-prepare` が `research-lock.json` と入力を照合します。Minecraft の所有・認証は公式 Launcher 側で行います。

公式 manifest から 26.3 の server jar を取得する例です。ダウンロードする前に Horizon checkout の `research-lock.json` に一致する jar の SHA-1 を確認し、保存後も検証します。

```powershell
$work = Join-Path $HOME 'horizon-work'
$horizon = Join-Path $work 'horizon'
$lock = Get-Content -Raw -LiteralPath (Join-Path $horizon 'research-lock.json') | ConvertFrom-Json
$versions = Invoke-RestMethod 'https://piston-meta.mojang.com/mc/game/version_manifest_v2.json'
$entry = $versions.versions | Where-Object { $_.id -eq $lock.minecraft.version } | Select-Object -First 1
if (-not $entry) { throw "Minecraft $($lock.minecraft.version) is absent from official manifest" }
$version = Invoke-RestMethod $entry.url
if ($version.downloads.server.sha1 -ne $lock.minecraft.server_sha1) { throw 'Official server hash does not match research-lock.json' }
$downloadDir = Join-Path $horizon '.local\downloads'
New-Item -ItemType Directory -Force -Path $downloadDir | Out-Null
$serverJar = Join-Path $downloadDir 'server-26.3.jar'
Invoke-WebRequest -Uri $version.downloads.server.url -OutFile $serverJar
if ((Get-FileHash -LiteralPath $serverJar -Algorithm SHA1).Hash -ne $lock.minecraft.server_sha1) { throw 'Downloaded server jar hash mismatch' }
& (Join-Path $work 'horizon-mods\tools\setup-windows-tconstruct.ps1') -Prepare -ServerJar $serverJar
```

標準以外の Launcher ディレクトリを使う場合は `-MinecraftDir 'C:\path\to\minecraft'` を追加します。準備スクリプトは Horizon、`horizon-ctl`、Mod ビルダー、両 Mod をビルドし、リソースを `horizon\.local\horizon-client\26.3` に生成します。リソースが準備済みなら `-Prepare -ServerJar` を省いて再ビルドできます。checkout が隣接していなければ `-Horizon 'C:\path\to\horizon'` を指定し、手動起動にも同じ本体とリソースを指定してください。

## 手動で鉄鉱石と耐久を確認

```powershell
$work = Join-Path $HOME 'horizon-work'
$play = Join-Path $work "horizon-mods\Tinkers' onstruct\tools\play-item-state.ps1"
& $play -Inspect
```

新しい専用 Creative ワールドが開きます。T キーでチャットを開き、次を入力します。

```text
/tconstruct specimen flint wood wood 0
/tconstruct_tool activate
/tconstruct_tool inspect
```

生成された木のピッケルに見える個体を手に持って `activate` し、`inspect` の `damage=0`、`durability=85`、`hooks=mining` を確認します。Creative のまま `/setblock ~ ~ ~2 minecraft:iron_ore`、続いて `/gamemode survival` と入力し、鉄鉱石を採掘します。鉄の原石がドロップし、再度 `/tconstruct_tool inspect` すると `damage=1` です。vanilla の耐久バーはこのコア耐久を表しません。二本目の個体を作り、持ち替えて採掘するとそれぞれの damage が別に増えます。Creative で採掘した場合と硬度 0 のブロックは消費しません。damage 85 で broken となり、さらに採掘してもツールは消えません。

終了後に同じワールドを再開するには `& $play -Inspect -ResumeLatest`、in-frame 方式は `& $play -Inspect -InFrame`。リソースや本体が別の場所なら `-Client 'C:\...\horizon-client.exe' -Resources 'C:\...\26.3'` を追加します。`-ValidateOnly` は起動前に入力ファイルを確認します。ログとワールドの場所はスクリプトが表示します。[Linux 実ゲームの前後画像](images/tconstruct-m3b4/README.md) も参照できます。

## 自動回帰

デスクトップのログイン中に実行します。専用ワールドとログを作り、統合サーバー・in-frame の採掘と保存復帰、二人接続、専用サーバーを順に試します。複数のゲームが起動する場合があります。

```powershell
$test = Join-Path $HOME 'horizon-work\horizon-mods\tools\test-windows-tconstruct.ps1'
& $test -Mode all
# 個別実行: integrated / in-frame / players / dedicated
& $test -Mode integrated
```

各試験の `RUN=...` に `report.json` と `game.log` が残ります。失敗時はその内容を確認してください。Windows 実機での新しい手順・自動試験の実行結果は未確認です。Cloud Linux での成功を Windows の成功とは扱いません。

Cloud Linux では PowerShell 7.5.4 でスクリプト構文、スペース入り checkout パスとテスト選択を確認し、PowerShell の自動試験入口から integrated の 9 項目を実行して成功しました。Linux 実行ファイルと仮想ディスプレイによる確認であり、Windows 向けバイナリや Windows の GUI 動作を確認した結果ではありません。
