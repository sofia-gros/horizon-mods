# 検証・配布・トラブル対応

## コードModの検証

対象Modのディレクトリで実行します。

```powershell
cargo fmt --check
cargo horizon-mod test
cargo horizon-mod check .\target\horizon-mod\hello_horizon.hmod
```

`hello_horizon` は対象manifestの `id` に置き換えます。
`test` はビルド後に `tests/*.toml` をゲームと同じsandboxで実行します。ゲーム本体やウィンドウは不要です。
ファイル名の一部を指定すると絞り込めます。`--debug` は非最適化ビルド、`--hmod <path>` は既存成果物の検証です。

```powershell
cargo horizon-mod test hello
cargo horizon-mod test --hmod .\target\horizon-mod\hello_horizon.hmod
cargo horizon-mod check .\target\horizon-mod\hello_horizon.hmod --json
```

生成テンプレートの振る舞いを検証する例:

```toml
[[step]]
expect.commands = ["hello"]

[[step]]
command = "hello world"
expect.chat = ["Hello world! (1 so far)"]

[[step]]
reload = true
command = "hello again"
expect.chat = ["Hello again! (1 so far)"]
```

テストの `command` は `/` を付けません。メモリ上のカウンターはreloadで初期化されます。
各シナリオは専用の一時コピーで実行され、storageは空から始まります。
同一シナリオ内のreloadではstorageが残るため、永続設定の検証に使えます。

snapshotを読むModは `[player]`、`[clock]` と各stepの変更値、`ticks`・`frames`・`chat` で入力を作ります。
タイトル画面の挙動は `title-screen = true` で確認できます。
HUDには `expect.panel` / `expect.corner`、コマンド応答には `expect.feedback` / `expect.error` が使えます。
ゲームAPIから独立した計算や変換は、必要に応じて通常の `cargo test` で検証します。

`check` はmanifest・内容・import権限を検査し、コードがあれば40 ticks・120 frames・チャット・登録コマンドを試行します。
個別の仕様に対するシナリオテストや実画面の確認も行います。
コンテンツのみのModでは `check` とゲーム内確認を使います。

## ゲーム内確認

1. `cargo horizon-mod run` で対象Modを起動し、画面・操作・コマンドを確認する。
2. タイトル画面とワールド内を行き来し、プレイヤー不在時や再読み込みを確認する。
3. 再ビルド時の状態初期化・設定保存を確認する。
4. 通常インストールでも有効化・無効化を試し、複数ModのID・コマンド・HUD・名前空間の競合を確認する。
5. 内容を変更するModはテストワールドで保存・再起動後まで確認する。

## 通常インストールと配布

ゲームディレクトリの `options.txt` と同じ階層にある `horizon-mods/` に `.hmod` を配置します。
この開発用フォルダに置いただけではゲームへインストールされません。
Options > Modsで必要な権限を確認して有効化します。通常のModは初期状態では無効です。
有効状態はゲーム側の `horizon-mods.txt` に保存されます。

配布時はバージョンを更新し、SDK・CLI・Rust・Horizonの検証対象を記録します。
ビルド・シナリオテスト（コードMod）・`check`・ゲーム内確認を済ませた `.hmod` を配布します。
成果物のSHA-256はPowerShellの `Get-FileHash <path> -Algorithm SHA256` で記録できます。

Horizon公式レジストリへの申請は、公開ソースと利用可能なライセンスを用意し、
[公式の投稿フォーム](https://playhorizon.dev/mods/submit) の手順に従います。
上流ガイドでは、投稿後に配布用レジストリへのPRが作られ、検証とレビューを経て公開されます。
この開発用リポジトリのディレクトリ構成と、レジストリの登録用metadataは別です。

## よくある問題

| 症状 | 確認・対応 |
| --- | --- |
| `cargo horizon-mod` が見つからない | CLIのインストール結果、CargoのbinへのPATHを確認 |
| Rustバージョン不足 | CLIの推移的依存まで確認。調査対象のランタイムはRust 1.95以上 |
| Wasm targetがない | `rustup target add wasm32-unknown-unknown` |
| `horizon-mod.toml` が見つからない | ルートではなく対象Modの中で実行 |
| `uses capability ... missing` | SDKの使用機能とmanifestのcapabilitiesを揃える |
| experimentalのエラー | 使用する実験権限を `experimental` にも列挙 |
| ゲーム実行ファイルが見つからない | `--game` または `HORIZON_CLIENT` に本体の実行ファイルを指定 |
| コードModが起動しない | 本体のmods feature、API互換性、Options > Modsのエラー、ログを確認 |
| コマンドが呼ばれない | 登録名、vanillaや他Modとの名前衝突を確認 |
| ID・名前空間の重複 | 同時に有効なMod間で固有のID・名前空間にする |
| コンテンツModでtestが失敗 | コードなしModは `check` とゲーム内検証を使用 |
| ホットリロードで状態が消える | メモリ状態は初期化される。残したい設定はstorageへ保存 |

根拠: [テストランナー](https://github.com/ludovic111/horizon/blob/9e447a7e9203eefdf5204eaa3079be0a2ea6f4df/crates/horizon-mods/src/testing.rs)、[check実装](https://github.com/ludovic111/horizon/blob/9e447a7e9203eefdf5204eaa3079be0a2ea6f4df/crates/cargo-horizon-mod/src/check.rs)、[インストール・公開手順](https://github.com/ludovic111/horizon/blob/9e447a7e9203eefdf5204eaa3079be0a2ea6f4df/docs/mods.md)。
