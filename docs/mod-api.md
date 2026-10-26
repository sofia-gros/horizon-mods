# API・manifest

## コードModの仕組み

Rustの `cdylib` を `wasm32-unknown-unknown` 向けにコンパイルし、CLIがWebAssemblyコンポーネントに変換して `.hmod` に格納します。
`.hmod` はZIP形式で、コードModには `horizon-mod.toml` と `mod.wasm` が入ります。
Rustの `Mod` traitを実装し、`horizon_mod_sdk::export_mod!(型名)` で公開します。

`Mod::init()` は `Result<(Self, Registration), String>` を返し、状態と必要なフックを登録します。
`Registration::new()` に次のbuilderを追加します。

| builder | 呼び出し |
| --- | --- |
| `.command("name")` | `/name args` → `command(&mut self, name, args)` |
| `.on_tick()` | ワールド内で毎ゲームtick（20回/秒）→ `tick(&mut self, tick)` |
| `.on_frame()` | 描画フレームごと → `frame(&mut self, seconds)` |
| `.on_chat()` | 受信チャット → `chat(&mut self, text)` |

`command` の戻り値は `Ok(Some(text))` がフィードバック、`Ok(None)` が応答なし、`Err(text)` がエラー表示です。
登録名に `/` は付けません。vanillaのコマンドが優先され、Mod同士では先に読み込まれたものが優先されます。
自分のチャットコマンドの登録と、vanillaコマンドを実行する `command` capabilityは別の機能です。

## manifestの例

```toml
id = "coords_panel"
name = "Coordinates Panel"
version = "0.1.0"
description = "プレイヤーの座標を表示する"
authors = ["Your Name"]
api = "0.1"
side = "client"
capabilities = ["game-read", "hud"]
experimental = []
```

未知のフィールドや不正なIDは拒否されます。権限はコンポーネントの実際のimportと照合されます。
API `0.2` のコンテンツ登録には `registries` が必要です。`side = "server"` を書くだけではサーバーコードModになりません。
調査対象で受け付けるmanifest APIは `0.1` と `0.2` であり、将来の `0.3` は利用可能なAPIとして扱いません。

## 利用可能なcapability

| capability | SDKモジュール・用途 | 状態 |
| --- | --- | --- |
| 不要 | `log` / `info` / `warn` によるログ | 常時利用可 |
| `game-read` | `game`: プレイヤー位置・向き・体力・満腹度・dimension、ゲーム時刻 | API 0.1 |
| `chat` | `chat::show`: ローカルプレイヤーにチャットを表示 | API 0.1 |
| `hud` | `hud::set_panel` / `clear`: 保持型のテキストパネル | API 0.1 |
| `storage` | `storage`: Mod専用のkey/value保存 | API 0.1 |
| `command` | `command::run`: プレイヤーの権限でvanillaコマンドを予約実行 | API 0.1 |
| `registries` | JSONによるブロック・アイテム登録（WIT importなし） | API 0.2 |
| `ui` | 自分の画面・設定ページ・widget | 実験API |
| `render` | HUD図形・テキスト・FPS・zoom・minimapなど | 実験API |
| `input` | キー登録・入力poll・アイテム使用の取得・wheel取得など | 実験API |

実験APIは、使うものを両方の配列に列挙します。

```toml
capabilities = ["render", "input", "storage"]
experimental = ["render", "input"]
```

UIのイベントは `Mod::ui_select`、`ui_closed`、`ui_settings_opened` で処理します。
`input` のイベントは `input::poll()` で取得します。実験APIの型・引数は固定したSDKのWITを確認してください。
同じAPI番号でも実験APIはリリース間で変わる可能性があります。

## 設計上の制限

- WASIはなく、任意のファイル・ネットワーク・プロセスへ直接アクセスできません。保存は `storage` を使います。
- メモリ上限はModごとに64 MiB、パッケージ内の `mod.wasm` は32 MiB以下です。
- HUDは16行まで、各行256文字まで。チャット表示とvanillaコマンド予約は、それぞれ1呼び出しで8件までです。
- storageは最大64ファイル・合計4 MiB、1呼び出しの書き込みは最大1 MiBです。
- 目標処理時間はtickあたり1 ms、frameあたり0.1 ms以内。fuel超過やtrapでそのModが停止します。
- HUDのパネルは更新後に本体が描画を続けます。毎frame同じ内容を送る必要はありません。
- プレイヤーや時刻はsnapshotとして取得します。タイトル画面などでは `None` を処理してください。
- JavaのMixinのように本体をpatchできません。未提供のインベントリ・ワールド・ネットワークAPIを想定して実装しないでください。

根拠: [SDK実装](https://github.com/ludovic111/horizon/blob/9e447a7e9203eefdf5204eaa3079be0a2ea6f4df/crates/horizon-mod-sdk/src/lib.rs)、[WIT](https://github.com/ludovic111/horizon/blob/9e447a7e9203eefdf5204eaa3079be0a2ea6f4df/crates/horizon-mod-sdk/wit/horizon.wit)、[manifest検証](https://github.com/ludovic111/horizon/blob/9e447a7e9203eefdf5204eaa3079be0a2ea6f4df/crates/horizon-mods/src/manifest.rs)、[上流ガイドの制限値](https://github.com/ludovic111/horizon/blob/9e447a7e9203eefdf5204eaa3079be0a2ea6f4df/docs/mods.md)。
