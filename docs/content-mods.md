# コンテンツMod

## 作成とビルド

コードを書かず、vanillaのテンプレートを基にブロックやアイテムを追加できます。
CLIの導入後、作業場所のルートで実行します。

```powershell
cargo horizon-mod new glowing_gems --template content --name "Glowing Gems"
Set-Location .\glowing_gems
cargo horizon-mod build
cargo horizon-mod check .\target\horizon-mod\glowing_gems.hmod
```

manifestは次の構成になります。`Cargo.toml` と `src/` は生成されません。

```toml
id = "glowing_gems"
name = "Glowing Gems"
version = "0.1.0"
description = "発光するブロックを追加する"
authors = []
api = "0.2"
side = "both"
capabilities = ["registries"]
```

`side = "both"` は上流のコンテンツテンプレートの値です。これだけでサーバーフックやマルチプレイ同期の動作確認が済んだことにはなりません。
コードなしの `.hmod` では `mod.wasm` は不要です。ビルドツールはmanifest・assets・dataをまとめます。

## 生成される構成

```text
glowing_gems/
├── horizon-mod.toml
├── assets/glowing_gems/
│   ├── blockstates/glowing_amethyst.json
│   ├── models/block/glowing_amethyst.json
│   ├── items/glowing_amethyst.json
│   └── lang/en_us.json
└── data/
    ├── glowing_gems/
    │   ├── horizon/block/glowing_amethyst.json
    │   ├── loot_table/blocks/glowing_amethyst.json
    │   └── recipe/glowing_amethyst.json
    └── minecraft/tags/block/mineable/pickaxe.json
```

生成テンプレートにはブロック、モデル参照、表示名、loot、レシピ、採掘用tagが揃います。
調査対象では `recipe`、`loot_table`、`tags/block` のように単数形のディレクトリを使います。
古いMinecraft版向けのデータパック例をそのまま移さず、対象バージョンの形式に合わせます。

ブロック定義の例（`data/glowing_gems/horizon/block/glowing_amethyst.json`）:

```json
{
  "template": "minecraft:amethyst_block",
  "properties": { "hardness": 2.0, "light": 7 }
}
```

この例は生成テンプレートと同じ定義です。vanillaの既存ブロックを基にします。
アイテムの登録定義は `data/<id>/horizon/item/*.json`、見た目のアイテム定義は
`assets/<id>/items/*.json` に置きます。この2種類は用途が異なります。
テンプレートはvanillaのモデル・テクスチャを参照でき、Minecraftのファイルを同梱する必要はありません。

## 名前空間と競合

基本は `assets/<id>/` と `data/<id>/` に置きます。
他に所有する名前空間が必要ならmanifestに `namespaces = ["extra_namespace"]` を宣言します。
ブロック・アイテムの登録定義は追加名前空間ではなく `data/<id>/horizon/` 配下です。

`minecraft`、`c`、`horizon` は所有する名前空間として宣言できません。
`data/minecraft/tags/` と `data/c/tags/` へのtag追加は認められます。
vanillaの通常ファイルを自由に上書きできるわけではありません。
tagの `replace: true` はそれ以前の内容を取り除くため、複数Modの組み合わせで確認します。
IDまたは所有する名前空間が重複すると、後から読む側のコンテンツが除外されます。

上流の上限は65,536ファイル、1ファイル16 MiB、合計512 MiBです。
独自アセットを追加する場合はライセンスと出典を記録します。

## 確認

コードのないコンテンツModは `cargo horizon-mod test` の対象にできません。
`check` で構造を検証した後、ゲームを再起動し、使い捨てのテストワールドで登録・表示・設置・破壊・loot・レシピを確認します。
保存後に再読み込みした結果と、他Modを有効にした場合の競合も確認してください。

根拠: [生成テンプレート](https://github.com/ludovic111/horizon/tree/9e447a7e9203eefdf5204eaa3079be0a2ea6f4df/crates/cargo-horizon-mod/template/content)、[コンテンツガイド](https://github.com/ludovic111/horizon/blob/9e447a7e9203eefdf5204eaa3079be0a2ea6f4df/docs/mods.md#content-mods-api-02)、[コンテンツ検証実装](https://github.com/ludovic111/horizon/blob/9e447a7e9203eefdf5204eaa3079be0a2ea6f4df/crates/horizon-mods/src/content.rs)。
