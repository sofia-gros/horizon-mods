# Horizon上流の追従記録

## M3b1: 読み取りAPIとquiet commandの統合

`3edb037e` で本流 `8027cb41` のMod index（download count / rating）とlauncher更新を統合。続いて `93b17ebd` で本流 `800b9156` を取り込み、`command.run-quiet`（level 3）、proxy設定・credentialのメッセージ抑制等を反映しました。作業中の変更はstashへ退避してから戻し、既存の退避内容も保持しました。

追加した読み取り専用 `items-read.main-hand` は、quiet commandと衝突しない**forkのlevel 4**です。汎用コードのみを本体 `235cac33f056efaeb316e5e309b767e14498ab79` へcommit・push。TConstructの素材・schema・計算はMod側です。commit前の最終fetchは `9422f698`、その後に追加のMod関連更新はありませんでした。
Mod側のcommit前にも `1f88194b` までfetchして再確認。追加のSDK・runtime・server接続・registry・launcher更新はありませんでした。

統合後のMod library 34件、API参照4件、sandbox 29件（負荷測定1件ignored）、mods有効のclient lib/binsとSDK/exampleのClippy、Mod無効のclient checkが成功。launcherの新UIやproxy設定の実操作、registry web service自体の試験は今回のゲームMod検証には含めていません。ゲームでの個体読み取り結果は [M3b検証](tconstruct-m3b-verification.md) に記録します。

## M3a開始時

`codex/tconstruct-item-hooks` でM2 `efeb2118` から分岐し、upstream `749a8a2a` をmerge commit `0dd64e68` で統合。API level 2とraycast snapshotを含みます。`game/mods.rs` の競合はM2のplayer snapshotと上流のview-hitを両方保持して解消しました。library 32件、API reference 4件、sandbox 27件（別途負荷測定1件ignored）が成功。

再開時は `034c0f02` までfetch。後続差分にWIT・SDK・runtime・package・server Mod契約の追加はなく、M3aの基準は検証済みの `749a8a2a` 取り込み版とします。itemの操作・性能hookが追加されたとは扱いません。
ビルド中・commit前にも再確認し、最終確認は `c65338b3`。追加のMod契約・SDK・runtime・server接続・launcher更新や、今回のstack送受信処理の更新はありませんでした。

## M2ブランチでの後続統合

`codex/server-mod-game-integration` では、upstream `b8027d4b`（API level対応を含む）を基準に、未マージのserverゲーム接続PR #1894をmerge commit `20e2faf9` で取り込みました。下記の「ゲーム未接続」はmainの初回調査の記録です。M2の進捗・版・試験範囲は [M2検証記録](tconstruct-m2-verification.md) を参照してください。

作業中の再fetchで本流にPR #1837のマージ `4fbb0cde` を確認。M2へ取り込んだPR #1894は同じserver test-scriptコミットを既に含み、追加の契約差分はありません。上流の型付きコマンド・world capabilityの未マージPRは取り込んでいません。

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
