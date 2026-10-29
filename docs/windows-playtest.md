# Windows 11: Mixins 版を試す

**未検証:** この手順は Linux で検証した現行 Mixins ビルドを Windows PowerShell に合わせたものです。Windows 実機でのビルド・起動・描画はまだ確認していません。原作のランダムな余りが発生するリサイクルは、現在の非 Unix 実装では取得できません。Part Builder と Station は未完成です。エラーが出た場合は止めて、エラー全文を共有してください。

旧退避 `92f696b` の PC 設定には `A:\Project` と、Horizon Launcher の準備済みリソース `C:\Users\metal\AppData\Roaming\horizon-launcher\versions\v0.2.66\.local\horizon-client\26.3` が記録されています。下ではその設定を利用しますが、移動・バージョン変更されていたら変えてください。旧 Wasm 用 `.hmod` や旧セットアップスクリプトは使いません。Mixins は Horizon のソースを変換し、**別の実行ファイル**をビルドします。Minecraft Java や Forge にそのまま読み込ませる Mod ではありません。

必要なもの: Windows 11、PowerShell 7、Git、Python 3.12 以降、Rust stable の MSVC toolchain、Visual Studio Build Tools の「C++ によるデスクトップ開発」、Vulkan 対応 GPU、準備済み Horizon 26.3 リソース、自分の Minecraft 1.20.1 client jar と Forge 1.20.1-47.4.0 universal jar。Forge jar は [公式 Maven](https://maven.minecraftforge.net/net/minecraftforge/forge/1.20.1-47.4.0/forge-1.20.1-47.4.0-universal.jar) から取得できます。クライアント jar と Forge jar は同梱されず、ツールが固定ハッシュを検証します。リソース作成はシンボリックリンクを使うため、Windows の開発者モードを有効にするか、リンクを作成できる権限が必要です。

以下の `$MinecraftJar` と `$ForgeJar` は **自分の実ファイルへ変更**してください。記録された Horizon Launcher は `A:\Download\ABD\Compressed\horizon-launcher-windows-x64\horizon-launcher.exe` です。必要なら Launcher で 26.3 を準備してください。Prism の設定や既存ワールドは変更しません。

```powershell
$ErrorActionPreference = 'Stop'
$Root = 'A:\Project\tconstruct-mixins-playtest'  # 新規専用フォルダー
$Resources = Join-Path $env:APPDATA 'horizon-launcher\versions\v0.2.66\.local\horizon-client\26.3'
$MinecraftJar = 'C:\REPLACE\1.20.1.jar'            # 要変更
$ForgeJar = 'C:\REPLACE\forge-1.20.1-47.4.0-universal.jar' # 要変更

foreach ($path in @($Resources, $MinecraftJar, $ForgeJar)) {
    if (-not (Test-Path -LiteralPath $path)) { throw "入力が見つかりません: $path" }
}
if (-not (Test-Path -LiteralPath (Join-Path $Resources 'horizon-resources.json'))) {
    throw 'Horizon 26.3 の準備済みリソースを指定してください。'
}
if (Test-Path -LiteralPath $Root) { throw "既存データを保護するため中止: $Root。別の新しい名前を指定してください。" }
New-Item -ItemType Directory -Path $Root | Out-Null
Set-Location -LiteralPath $Root
function Run($program, [string[]]$arguments) {
    & $program @arguments
    if ($LASTEXITCODE -ne 0) { throw "$program failed: $LASTEXITCODE" }
}
Run git @('clone', '--branch', 'codex/tconstruct-stations', 'https://github.com/sofia-gros/horizon-mods.git', 'horizon-mods')
Run git @('clone', 'https://github.com/horizon-rust/horizon.git', 'horizon-official')
Run git @('-C', 'horizon-official', 'checkout', '--detach', 'b6c936f82b08798057161bdfa577e3aac53fb91e')
Run git @('clone', 'https://github.com/horizon-rust/horizon-mixins.git', 'horizon-mixins')
Run git @('-C', 'horizon-mixins', 'checkout', '--detach', '0500ff24aa5933fca95a877ef41125353c4b4fe3')
Set-Location (Join-Path $Root 'horizon-mods')
Run git @('rev-parse', 'HEAD') # 診断のため表示された SHA を控える
Run py @('-3', 'tools/stage.py', '--horizon', '../horizon-official', '--mixins', '../horizon-mixins')
Run cargo @('build', '--locked', '--manifest-path', 'target/modded/Cargo.toml', '-p', 'horizon-client', '--bin', 'horizon-client', '--no-default-features', '--profile', 'fast', '--target-dir', 'target/build')
Run py @('-3', 'tools/resources.py', $Resources, 'target/item-resources')
$Game = Join-Path $Root 'horizon-mods\target\build\fast\horizon-client.exe'
Run $Game @('--tconstruct-prepare', 'target/item-resources', 'content', 'target/resources')
Run py @('-3', 'tools/source_data.py', $MinecraftJar, 'target/resources/minecraft-original-pack')
Run py @('-3', 'tools/source_data.py', '--forge', $ForgeJar, 'target/resources/forge-original-pack')
```

初回実行には時間と空き容量が必要です。`stage.py` は固定した二つの Git commit を検証し、既存の出力があると停止します。中断後は既存データを消さず、別の新しい `$Root` を選んでください。資源 overlay は元リソースへのリンクを含むため、元の場所を維持します。

## 起動とゲーム内操作

同じ PowerShell セッションで実行します。保存先は新規専用フォルダーの `game\saves` です。ゲーム終了後、同じコマンドで再開できます。

```powershell
$GameResources = Join-Path $Root 'horizon-mods\target\resources'
$GameHome = Join-Path $Root 'game'
New-Item -ItemType Directory -Force -Path $GameHome | Out-Null
$env:HORIZON_HOME = $GameHome
Set-Location -LiteralPath $GameHome
& $Game --resources $GameResources --world 'TConstruct-Playtest' --seed 42 --creative
```

`T` でチャットを開き、次を一行ずつ入力して作業台と材料を入手できます。コマンドは試遊のための補助です。

```text
/give @s tconstruct:part_builder 1
/give @s tconstruct:pattern 8
/give @s minecraft:oak_planks 16
/give @s minecraft:flint 16
/give @s minecraft:calcite 16
/give @s tconstruct:tool_handle_cast 1
```

Part Builder を置いて右クリックし、Pattern と板材を入力、一覧から Tool Handle を選び、プレビューと結果を確認して取り出します。検証済み Linux 実行では板材 1 枚から木の柄ができ、保存・再開後も材料情報が残りました。Pattern と calcite を入力し、Helmet Plating を選ぶと石製 Helmet Plating を作れます（calcite 3 個を消費）。石製防具プレート 4 種を登録しています。Linux の検証用ビルドで helmet の制作と保存を確認していますが、正式変換版・Windows・残る 3 種と詳細 tooltip は未検証または未実装です。Tool Handle Cast を Pattern の代わりに入れると再利用可能な金型として柄を選べます。Linux の検証用ビルドでは板材消費後もこの金型が残りました。鋳造による自然な入手、金型の詳細 tooltip、他の金型と Windows は未実装または未検証です。材料や候補を変えて試せますが、未登録物・未対応レシピがあります。原作の革防具・亀の甲羅の耐久値もリサイクル共通処理へ追加しています。Linux の検証用ビルドでは革ブーツとコマンド投入の venom bottle から革と空瓶を取得し、保存・再開を確認しました。正式 Mixins ビルドと他の防具、Windows でのこの操作は未検証です。複数結果のランダム余り、隣接 inventory 連携、詳細表示、完全な道具組立などは未完成です。[現状の一覧](../README.md) と [対応状況](roadmap.md) を参照してください。

実行時のエラー調査には、表示された Git SHA、失敗したコマンド、エラー全文、Windows・Python・Rust のバージョンを使ってください。Linux の成功は Windows 成功を意味しません。
