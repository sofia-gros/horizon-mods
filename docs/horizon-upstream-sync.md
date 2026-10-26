# Horizon上流の追従記録

## 確認のタイミング

開発開始時、API設計・実装の直前、ロードマップの区切り、commit・push前に `upstream/main` をfetchする。
上流更新が速いため、前回の調査結果だけで本体APIの新設を決めない。無人の定期監視を設定したものではなく、作業中の確認手順とする。

```powershell
& 'A:\Project\horizon-mods\tools\check_horizon_upstream.ps1'
```

比較用スクリプトはfetch・差分表示だけを行う。WIT、SDK、manifest、runtime、package/check、client、integrated/dedicated server、launcherを確認し、必要な更新を取り込んで対象試験を実行する。未コミット変更を保持し、重なる場合は退避・統合してから試験する。force pushは行わない。

## 2026-10-26: Server Mod API 0.3

- 直前の本体: `2a0f4e39a8b0b07d3da133faf0eb8d934a412e44`。
- 通知された変更: [`c9f554d1c4842ce6b408f5db95479879df545cd5`](https://github.com/ludovic111/horizon/commit/c9f554d1c4842ce6b408f5db95479879df545cd5)。
- 取り込んだマージ: `88b7bfaf`（upstream PR #1833）。fast-forwardで統合。既存のitem custom-data往復修正2ファイルは保持。

| 対象 | ソースで確認した実装 | TConstructへの影響 |
| --- | --- | --- |
| manifest / WIT | `side = "server"`, `api = "0.3"`, world `horizon-server-mod` | 実験的契約を再利用。独自のrole別ABIを新設しない |
| SDK | `server::ServerMod`, `Registration`, `export_server_mod!` | server側adapterの土台がある |
| imports | log / storage / players（online、tell、broadcast） | player情報と通知が可能。item操作やinventory APIは含まれない |
| exports | init、tick、join/leave、chat、command、block broken/placed | lifecycle観測が可能。blockイベントは変更を取り消す契約ではない |
| runtime | `Mods::load_for(Runs)` とclient/server別command所有 | ウィンドウとサーバーで実行対象を分ける土台がある |
| CLI / sandbox | server componentの試行、welcome例、role分離試験 | ゲーム接続前にもserver契約を検証できる |
| ゲーム | `game/mods.rs` は `Runs::Client`、snapshotのplayersは空 | server Modは「別の場所で実行」の扱い。実ワールドで動く状態には未到達 |

確認した一次ソースは本体の `crates/horizon-mod-sdk/wit/horizon.wit`、`crates/horizon-mod-sdk/src/lib.rs`、`crates/horizon-mods/src/{manifest,runtime}.rs`、`crates/horizon-client/src/game/mods.rs`、`docs/mods.md` のServer mods節。

M2の残作業は、上流基盤へのゲーム側接続、dev packageのserver側引き渡し、全実行方式でのlifecycle・player snapshot・出力配送・保存/終了の検証。M3以降のツール個体操作・採掘性能・耐久・menu・Block Entity・fluid APIは、この変更では追加されていない。

### 実施した検証

`A:\Project\horizon` で実行:

```powershell
cargo test --locked -p horizon-mods --features runtime,pack,index --lib --test sandbox --test api_reference
```

結果: library 28件、API reference 3件、sandbox 25件成功。sandboxの負荷測定1件は既定でignored。`a_server_mod_runs_only_where_the_world_does` を含む。SDKのclient例・server例 `welcome` もWasmへビルドされた。これは実ゲーム側のserver接続試験ではない。
CLIも `cargo test --locked -p cargo-horizon-mod --bin cargo-horizon-mod` で3件成功。

再fetchで上流 `31442769`（PR #1970、プレイヤーのdimension移動中の死亡処理修正）を確認。差分は `game/lan_actions.rs` のみで、今回のMod契約・SDK・runtimeの追加変更はなかった。本体はMod対応マージ `88b7bfaf` に固定して検証した。

### ユーザーがゲームで確認できる範囲

今回の取り込みだけではserver Modのゲーム内試験はまだできない。`welcome` を導入してもjoin通知や `/online` の動作確認にはならない。既存client診断Modの手順と、将来のserver接続完了時の手順を分けて案内する。M2は未完了のままとする。
