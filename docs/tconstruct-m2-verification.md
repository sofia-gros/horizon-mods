# M2 サーバーMod接続・検証

2026-10-26、ユーザーから実機で案内した期待結果すべてに一致したとの報告を受領。対象はM2の本体 `efeb2118` / Mod `3626730` と直前に案内した手動起動・診断手順。未試験の組み合わせまで確認済みとは扱わない。

本体とMod開発場はともに `codex/server-mod-game-integration` ブランチ。上流への新規PRは作成していない。

基準本体はupstream `b8027d4b`（API level対応を含む）。既存の [PR #1894](https://github.com/ludovic111/horizon/pull/1894) と、その依存の [#1837](https://github.com/ludovic111/horizon/pull/1837) を統合した検証ブランチで、upstream/mainの公開済み機能とは区別する。初期の重複実装とM3用custom-data変更は別々のstashで保持。

独自の追加差分は、server Mod tickを全dimension処理後に一度だけ呼ぶこと、blockイベントに発生時のdimensionを保持すること、操作APIのchatを手入力と同じclient Mod判定へ通すこと。

本体の統合commitは `20e2faf9`、dimension/tick修正は `df87aa3f`、control chat修正は `efeb2118`。専用ブランチをforkへpush済み。検証したgameのSHA-256は `46534b7c085461e61da6726ca4f79770fb80ecf85ac131d2e2ded3570f490ebe`、probe成果物は `62ce64972e9e023d22934377126988492e01453d62af0ac889fce9cf884cc7d1`。

## ビルド

各コマンドの成功を確認して次へ進む。

```powershell
Set-Location -LiteralPath 'A:\Project\horizon'
git branch --show-current
cargo build --locked --profile fast --no-default-features --features mods -p horizon-client --bin horizon-client --bin horizon-ctl -j 2
cargo build --locked -p cargo-horizon-mod
cargo test --locked -p horizon-mods --features runtime,pack,index --lib --test sandbox --test api_reference
Set-Location -LiteralPath "A:\Project\horizon-mods\Tinkers' onstruct\probes\server-lifecycle"
cargo build --locked --release --target wasm32-unknown-unknown
& 'A:\Project\horizon\target\debug\cargo-horizon-mod.exe' horizon-mod build
& 'A:\Project\horizon\target\debug\cargo-horizon-mod.exe' horizon-mod test
& 'A:\Project\horizon\target\debug\cargo-horizon-mod.exe' horizon-mod check target/horizon-mod/tconstruct_probe.hmod --json
```

M1のclient診断Modもbuild済みである必要がある。ProbeはMod内の独立Cargo projectであり、root workspaceは作らない。SDKは検証済み `88b7bfafc01c7c475548d4379845ff4a9f944dc0` に固定、manifestは実験的なAPI 0.3 / server / players・storage、必要API levelは1。TConstructの実ツールを追加するModではない。

## ユーザーの起動手順

```powershell
Set-Location -LiteralPath "A:\Project\horizon-mods\Tinkers' onstruct"
& .\tools\play-server-probe.ps1
```

このブランチの `target/fast/horizon-client.exe` とv0.2.66の準備済み26.3 resourcesで、統合サーバーモードのGUIを開く。既存セーブを使わず `.local/manual-M2-<固有ID>/` に専用データ・設定・ログを置く。in-frame比較は `-InFrame`。存在確認だけなら `-ValidateOnly`。

Tでチャットを開き、次を確認する。

| 操作 | 期待結果 |
| --- | --- |
| `/tprobe status` | JSONで `probe: M2`、`joins: 1`、onlineにactorが1人 |
| 少し待ってもう一度status | ticksが増える。joinsは1のまま |
| `/tprobe tell` | `M2 targeted reply` と `tell sent` |
| `/tprobe broadcast` | `M2 broadcast reply` と `broadcast sent` |
| `/tprobe error` | 赤い `M2 expected diagnostic error` |
| 通常のチャットを送りstatus | chatsが増え、lastに送信文字列 |
| ブロック1個を破壊してstatus | brokenが1増え、lastにdimension・座標・元のstate |
| creative inventoryでブロックを持ち1個設置してstatus | placedが1増え、lastに設置state |
| `/tconstruct stats wood wood wood` | M1と同じ耐久60、採掘2.000、攻撃1.000、速度1.200 |

ticksは実時間で変わる。`duplicate_ticks` は0が期待値。bootsは同じprobeファイルに対するstorageの起動回数。手動・自動起動スクリプトはprobeファイルも専用の実行フォルダへコピーするため、新規実行のbootsは1になる。現行上流のstorageはModファイル隣の専用フォルダであり、ワールド単位ではない。TConstruct個体やレシピをそこへ保存する実装はしていない。終了はEscメニューから。

## 自動試験・結果

開発場のrootで `A:\miniconda3\python.exe -B tools/probe_horizon_server_mods.py --mode MODE`。MODEはintegrated、in-frame、dedicated、lan-integrated、lan-in-frame。全childは新規home・cwdを使い、結果は `.local/server-mods-tests/`。専用サーバーは試験フォルダだけにeulaとoffline-mode設定を作り、loopbackに接続する。

- Mod基盤: library 32件、API reference 4件、sandbox 26件成功。負荷測定1件ignored。
- 本体: Mod無効のcheck、Mod有効のgame bin試験120件（3件ignored）、clippy（警告拒否）、fmt、public tree検査に成功。Windowsの初回test依存ビルドが中断したため、再実行は `RUST_MIN_STACK=16777216` と `-j 1` を指定した。
- Probe: Wasm build、成果物check、10ステップのscenario、Wasm clippy成功。
- integrated: 14項目成功（`integrated-006z1r2u/report.json`）。破壊・設置、Netherへのplayer dimension更新、pause/resume、client診断を含む。
- in-frame: 13項目成功（`in-frame-gmtq8cpv/report.json`）。破壊・設置、pause/resume、client診断を含む。Nether移動は下記制限のため対象外。
- dedicated: 7項目成功（`dedicated-yndtamlf/report.json`）。通常権限のguestからstatus・chat・targeted reply・error、M1 client診断、正常終了を確認。破壊・設置はこのモードでは未試験。
- LAN integrated: 10項目成功（`lan-integrated-bo850czk/report.json`）。他人へのtell漏洩なし、broadcast、leaveを確認。ホストがNether、ゲストがOverworldという2つのactive dimensionでも、同一tickのcallbackは一度だけ。
- LAN in-frame: 9項目成功（`lan-in-frame-vlpmohx0/report.json`）。通常権限guestの基本callback・出力分離・leaveを確認。
- 合計: 5構成53項目。各reportは `.local/server-mods-tests/` 以下。すべての操作・dimension・権限の組み合わせを網羅する試験ではない。

## 制限

実ツール・個体性能・採掘性能・耐久・修理・menu・機械はM3以降。blockイベントはプレイヤー操作の観測で、取消や性能変更を行わない。爆発・piston・setblockは対象外。型付きコマンドツリーとworld capabilityは別の未マージPRであり、利用可能と扱わない。

in-frame + test-terrainのNether移動では、dimensionは変わるがプレイヤー復帰・lighting完了が確認できず、Modコマンドへ到達しなくなった。M2導入前のインストール済みv0.2.66をModなしで起動しても、移動20秒後に同じ `dimension=the_nether / player=null` を確認（`baseline-v66-no-mods-fw8dpq7k/report.json`）。したがってM2固有の症状とは考えていないが、根本原因は未特定。M2試験はこの組み合わせを成功として数えず、dimensionの観測はintegratedで検証する。in-frameのNether利用を保証しない。
