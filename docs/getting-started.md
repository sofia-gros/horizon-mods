# 開発環境と作成手順

## 準備

必要なものはGit、Rust（Cargo / rustup）、WindowsのRustネイティブビルド用ツールチェーン、Horizonです。
WindowsではRustのMSVCツールチェーンに対応するVisual Studio Build ToolsのC++ビルドツールを用意します。
ModはWasmですが、開発CLI自体はネイティブでビルドされます。

調査対象の `horizon-mods` クレートは **Rust 1.95以上**を要求します。
上流READMEやCLIのpackageにある低い最小バージョンだけを見て環境を選ばないでください。
ゲームを遊ぶためのMinecraftのバージョン・初期準備は、使用するHorizonリリースの案内に従います。
調査対象の案内はJava Edition 26.3と初期準備用Java 25です。
ゲームなしでもコードModのシナリオテストは実行できます。

## ツールの導入

PowerShellで実行します。以下は調査したコミットにCLIを固定する例です。

```powershell
rustc --version
cargo --version
rustup target add wasm32-unknown-unknown
cargo install --git https://github.com/ludovic111/horizon --rev 9e447a7e9203eefdf5204eaa3079be0a2ea6f4df --locked cargo-horizon-mod
```

別のHorizonリリースに合わせる場合は、そのリリースに対応するコミットへ変更します。
最新stable Rustでも依存が要求するバージョンを満たさない場合は、上流が指定するツールチェーンを確認してください。
`wasm32-wasip1` ではなく **`wasm32-unknown-unknown`** を使います。

## コードModを作る

作業場所のルートで実行します。`hello_horizon` は新しく作るModの名前です。

```powershell
Set-Location A:\Project\horizon-mods
cargo horizon-mod new hello_horizon --name "Hello Horizon"
Set-Location .\hello_horizon
```

`Cargo.toml`、`horizon-mod.toml`、`src/lib.rs`、`tests/hello.toml`、`.gitignore` が生成されます。
生成時のSDK依存は追従するGit依存なので、最初のビルド前に次のように固定します。

```toml
[dependencies]
horizon-mod-sdk = { git = "https://github.com/ludovic111/horizon", rev = "9e447a7e9203eefdf5204eaa3079be0a2ea6f4df" }
```

```powershell
cargo horizon-mod build
cargo horizon-mod test
cargo horizon-mod check .\target\horizon-mod\hello_horizon.hmod
```

初回ビルドでできる `Cargo.lock` は管理対象です。
標準テンプレートは `/hello world` にチャットで応答し、呼び出し回数を数えます。
コマンドを変更したら、生成されたシナリオテストも変更してください。

## ゲームで試す

Horizonランチャーでゲームを準備し、実際の `horizon-client` 実行ファイルを指定します。
以下のパスは例なので、自分の環境に置き換えてください。

```powershell
$env:HORIZON_CLIENT = 'C:\path\to\horizon-client.exe'
cargo horizon-mod run
# 明示指定する場合
cargo horizon-mod run --game 'C:\path\to\horizon-client.exe'
```

`run` はビルド後、ゲームに `--dev-mod <成果物>` を渡します。
開発Modは有効状態で起動します。別ターミナルで同じModディレクトリに入り、編集後に
`cargo horizon-mod build` を実行すると、ゲームが約1秒ごとに更新を検出して再読み込みします。
メモリ上の状態は初期化されますが、`storage` に保存したデータは残ります。
コンテンツ登録の変更は、この仕組みだけで既存ワールドに反映されると仮定せず、ゲームを再起動して確認します。

ソースからゲームを作る場合、コードModには `--features mods` が必要です。
公式のリリースビルドでは有効ですが、自前の本体ビルドでは確認してください。
本体のビルド・Minecraftリソースの準備は、[上流README](https://github.com/ludovic111/horizon/blob/9e447a7e9203eefdf5204eaa3079be0a2ea6f4df/README.md) を参照してください。

コンテンツModの作成は [コンテンツMod](content-mods.md)、ゲームなしの検証は [検証手順](testing-and-release.md) へ進みます。

根拠: [CLI実装](https://github.com/ludovic111/horizon/blob/9e447a7e9203eefdf5204eaa3079be0a2ea6f4df/crates/cargo-horizon-mod/src/main.rs)、[ランタイム依存・Rust要件](https://github.com/ludovic111/horizon/blob/9e447a7e9203eefdf5204eaa3079be0a2ea6f4df/crates/horizon-mods/Cargo.toml)。
