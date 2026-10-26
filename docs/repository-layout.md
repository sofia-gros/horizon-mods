# 構成と運用

## 1ディレクトリ = 1Mod

Modの作成コマンドはこの作業場所のルートで実行し、以降のビルド・テストは作成したModの中で実行します。
共通ドキュメント以外のコード・アセット・テストは、そのModのディレクトリ内で完結させます。
ルートにCargo workspaceは設けず、Modごとに依存と出力を独立させます。

```text
{mod_name}/
├── README.md             # 目的・使い方・対象Horizon・検証結果
├── horizon-mod.toml      # ID・API・権限など
├── Cargo.toml            # コードを含む場合
├── Cargo.lock            # コードModの再現性のため管理
├── src/lib.rs            # コードを含む場合
├── tests/*.toml          # コードModのヘッドレステスト
├── assets/{mod_id}/      # モデル・表示名・独自テクスチャなど
├── data/{mod_id}/        # 登録定義・レシピ・lootなど
└── target/horizon-mod/{mod_id}.hmod  # 生成物、Git管理外
```

空の `src/` や `assets/` を全Modに作る必要はありません。コンテンツのみなら `Cargo.toml` も不要です。
出力先は通常の独立したCargo構成の場合です。`CARGO_TARGET_DIR` などを設定している場合は、ビルドが表示するパスを使用してください。

## 命名

- 新規Modのフォルダ名は `snake_case` を推奨します（この作業場所の規約）。
- manifestの `id` は `[a-z0-9_]{1,64}`。他のModと重複させません。
- 通常はフォルダ名とIDを一致させ、アセットの名前空間にも同じIDを使います。
- 生成ツールはフォルダ名の英数字を小文字にし、その他を `_` に置換してIDを作ります。Cargoのpackage名では `_` が `-` になります。
- チャットコマンドも他Modとの衝突を避ける名前を選びます。

## Modごとに記録すること

各Modの `README.md` に、少なくとも次を残します。

- 目的、操作方法、コマンド、設定、必要な権限とその用途。
- 動作確認したHorizonのバージョンまたはコミット、SDKのコミット、Rustのバージョン。
- ビルド・テスト・ゲーム内確認の手順と結果。
- 実験API、既知の制限、他Modとの依存・競合。
- ライセンス、独自アセットや外部素材の出典。

SDKは `Cargo.toml` のGit依存に `rev` を指定して固定し、`Cargo.lock` を管理します。
Modのリリース時はmanifestとCargoの `version` を揃えます。
ゲーム本体・Minecraftの抽出データ・テストワールドはこの作業場所の外に保持します。

根拠: 上流の [生成ツール](https://github.com/ludovic111/horizon/blob/9e447a7e9203eefdf5204eaa3079be0a2ea6f4df/crates/cargo-horizon-mod/src/main.rs)、[manifest実装](https://github.com/ludovic111/horizon/blob/9e447a7e9203eefdf5204eaa3079be0a2ea6f4df/crates/horizon-mods/src/manifest.rs)。ディレクトリ運用はこの作業場所独自の規約です。
