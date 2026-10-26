# M2 server lifecycle probe

TConstruct移植のホスト接続を検証する診断fixture。実ツールやレシピは追加しません。Mod内の独立Cargo projectで、rootのCargo workspaceには参加しません。

- ID: `tconstruct_probe`、version 0.1.0。
- manifest: **実験API 0.3**、server、players / storage。必要なhost API levelは1。
- SDK: `sofia-gros/horizon` の `88b7bfafc01c7c475548d4379845ff4a9f944dc0` に固定、Cargo.lockを管理。
- ゲーム: 本体の `codex/server-mod-game-integration`、`efeb211863af33be694aa5bc7cfdcffdce343588`。未マージの上流PR #1894を含みます。
- MIT。原作TConstructのコード・データはこのprobeには含めません。

このディレクトリで以下を実行します。CLIは本体でbuildしたものです。

```powershell
cargo build --locked --release --target wasm32-unknown-unknown
& 'A:\Project\horizon\target\debug\cargo-horizon-mod.exe' horizon-mod build
& 'A:\Project\horizon\target\debug\cargo-horizon-mod.exe' horizon-mod test
& 'A:\Project\horizon\target\debug\cargo-horizon-mod.exe' horizon-mod check target/horizon-mod/tconstruct_probe.hmod --json
```

`/tprobe status` はtick、join/leave、chat、break/place、actor、dimension、onlineをJSONで返します。`duplicate_ticks=0` が期待値。`request` は自動試験の応答識別用です。`tell` / `broadcast` / `error` で出力配送も観測できます。

`boots` はstorageで維持され、hot reload後の起動を観測するためのものです。テストランナーと手動起動スクリプトは試験フォルダへpackageをコピーして、その試験だけのstorageを使います。ゲーム内のツール状態保存ではありません。

ゲームの起動・操作・制限は [M2検証記録](../../../docs/tconstruct-m2-verification.md) を参照してください。in-frame + test-terrainのNether移動は保証しません。通常の統合サーバーを基準にしてください。
