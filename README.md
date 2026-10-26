# Horizon Mods

[Horizon](https://github.com/ludovic111/horizon) 用のModを複数開発する場所です。
共通ドキュメントは [`docs/`](docs/README.md)、各Modの実体はルート直下の `{mod_name}/` に置きます。

```text
horizon-mods/
├── docs/                  # 共通の開発ガイド・API・検証手順
├── {mod_name}/            # 1つ目のMod（作成時に追加）
│   ├── README.md
│   ├── horizon-mod.toml
│   ├── Cargo.toml         # コードModのみ
│   ├── src/               # コードModのみ
│   ├── tests/             # コードModのシナリオテストなど
│   ├── assets/            # 必要な場合
│   └── data/              # 必要な場合
└── {another_mod_name}/    # 別のMod
```

[Tinkers’ Construct移植](<Tinkers' onstruct/README.md>) を開発中です。進捗は [ロードマップ](docs/tconstruct-roadmap.md)、新規Modの作成は [開発環境と作成手順](docs/getting-started.md) を参照してください。
この作業場所自体は、上流の配布レジストリ `ludovic111/horizon-mods` とは別です。
