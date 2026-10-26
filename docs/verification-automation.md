# 検証自動化の調査とWindows実機確認

確認日: **2026-10-26（日本時間）**。
**Horizonの検証は自動化できます。Windowsの実機で、隔離したワールドをCLIで操作し、保存・終了・再起動後の保持まで確認しました。**
今回動かしたのはインストール済みリリースv0.2.66です。ローカルフォークの自前ビルドや、未実装のTinkers’ Constructの動作を検証した結果ではありません。

## 確認した配置と役割

| パス | 用途・確認結果 |
| --- | --- |
| `C:\Users\metal\AppData\Roaming\PrismLauncher` | Minecraftの入力元。assets・libraries・metaと26.3のinstanceを確認 |
| `A:\Download\ABD\Compressed\horizon-launcher-windows-x64\horizon-launcher.exe` | Horizonランチャー。実行ファイルの `--help` でCLI・MCPの存在を確認 |
| `C:\Users\metal\AppData\Roaming\horizon-launcher\versions` | v0.2.50 / .52 / .53 / .66 / .68を確認。各版にclient・ctl・prepare等がある |
| 同versionsのv0.2.52 / .53 / .66配下 | `.local/horizon-client/26.3/horizon-resources.json` が存在 |
| 同versionsのv0.2.68配下 | 調査時点で上記の準備済みresourceマーカーなし。最新番号だけを基準に検証版を選ばない |
| `C:\Users\metal\AppData\Roaming\horizon-launcher\linked-minecraft` | Prism等のファイルを公式launcher形式のversions / libraries / assetsへ対応させた入力元。26.3のmanifestとclient jarを確認 |
| `C:\Users\metal\AppData\Roaming\horizon-launcher\server-jars\26.3.jar` | 所有する26.3 server jar |
| `C:\Users\metal\AppData\Roaming\horizon` | 通常プレイの保存先。今回のゲーム起動では使用しない |
| `A:\Project\horizon` | 拡張するフォーク本体。今回の調査ではコードを変更していない |
| `A:\Project\horizon-mods\Tinkers' onstruct` | Mod側。まだ実行可能なModはない |

PrismLauncherはMinecraftの取得済みファイルの入力元であり、Horizon自身の通常セーブ先とは別です。
既存アカウント情報の内容は読み取っていません。

### 原版ファイルとJavaの確認

実ファイルをSHA-1で確認し、本体の `research-lock.json` にある値との一致を確認しました。

| 対象 | SHA-1 |
| --- | --- |
| Prismの `libraries/com/mojang/minecraft/26.3/minecraft-26.3-client.jar` | `e877b6a07acd633fb3bb475002175cec036e7b87` |
| Horizon launcherの `server-jars/26.3.jar` | `33680f5f2ac32864d6d7cf5e56a705fdb3e05f4c` |

Prismの `java/java-runtime-epsilon/bin/java.exe -version` はOpenJDK **25.0.1**でした。
本体checkoutに準備済みデータがないことは、素材となるMinecraftファイルが端末にないことを意味しません。
今回、既存の準備済みresourceでゲームを動かせたため、検証のためにMinecraftを再取得する必要はありませんでした。
ただし自前ビルドでのresource schema・exporter互換性は別途確認し、必要ならcheckoutの `.local/` へ再生成します。

## 実際に通った検証

新しいgame homeと作業ディレクトリを毎回作り、v0.2.66のclientと同じ版のctl・resourceを組み合わせました。
`--offscreen --test-terrain --creative --render-distance 2` を指定し、実GPUは **NVIDIA GeForce RTX 4060 / Vulkan**でした。

| 検証 | integrated（既定の内蔵サーバー） | `--in-frame` |
| --- | --- | --- |
| 起動・Windowsの制御endpointへ接続 | 成功 | 成功 |
| `wait-ready` と `state` のJSON取得 | 成功 | 成功 |
| `/setblock 0 80 0 minecraft:gold_block` → `block`で確認 | 成功 | 成功 |
| `/give @s minecraft:diamond 3` → inventoryで確認 | 成功、3個を観測 | 成功、3個をassert |
| screenshotでPNG生成 | 成功、実画像も目視確認 | 成功 |
| inventory画面を開く・menuのJSON取得 | このrunでは未実施 | 成功 |
| save → quit → プロセス終了コード0 | 成功 | 成功 |
| 同じテストworldを開き直す | 成功 | 成功 |
| 再起動後にgold blockとdiamondが残る | 成功、3個を観測 | 成功、3個をassert |

実行記録:

- [integratedのJSONレポート](A:/Project/horizon-mods/.local/automation/smoke-7u3oi89y/report.json)
- [in-frameのJSONレポート](A:/Project/horizon-mods/.local/automation/smoke-vgvrfpzy/report.json)
- [integratedのスクリーンショット](A:/Project/horizon-mods/.local/automation/smoke-7u3oi89y/smoke.png)

これらは `.local/` のGit管理外ファイルです。ワールド・ゲームログ・個々のCLI応答も同じrunディレクトリに保持しています。
レポートには実行ファイルのSHA-256と実行時刻、使用したパス、各stepの結果が記録されます。
今回のPNG確認は、描画が成立することの確認です。画像差分の許容誤差やモデルの正しさを判定する自動試験はまだ実装していません。

### 初回の失敗と回避

環境調査で既定の `python --version` がmiseのshim経由で再帰起動しました。そのプロセス群を停止し、実体の `A:\miniconda3\python.exe`（3.9.12）へ切り替えました。
その間の最初のゲーム起動はout of memoryで失敗しました。再帰プロセスを解消した後の再実行は成功していますが、これだけでメモリ不足の原因を完全に特定したとは扱いません。
再実行用スクリプトはPython 3.9で動作確認済みです。本体の上流開発ツールはPython 3.10以上を案内しているため、本体の全試験用には別途その要件を満たす実体を用意します。

## 再実行する方法

[probe_horizon_automation.py](../tools/probe_horizon_automation.py) を追加しました。Python標準ライブラリのみを使い、毎回新しい隔離ディレクトリを生成します。

```powershell
Set-Location -LiteralPath 'A:\Project\horizon-mods'
& 'A:\miniconda3\python.exe' -B .\tools\probe_horizon_automation.py
& 'A:\miniconda3\python.exe' -B .\tools\probe_horizon_automation.py --in-frame
```

既定では今回成功したv0.2.66を使います。`--client`、`--ctl`、`--resources`、`--output-parent` で上書きできます。
完成した自前ビルドへ切り替える場合の形式は次のとおりです。**このコマンドの対象ビルドとresourceはまだ用意・検証していません。**

```powershell
& 'A:\miniconda3\python.exe' -B .\tools\probe_horizon_automation.py `
  --client 'A:\Project\horizon\target\fast\horizon-client.exe' `
  --ctl 'A:\Project\horizon\target\fast\horizon-ctl.exe' `
  --resources 'A:\Project\horizon\.local\horizon-client\26.3'
```

スクリプトは終了コード0を成功、1を実行中の失敗として返します。起動前に必須ファイルがない場合は引数エラーになります。
CLIの起動・応答にtimeoutを設け、失敗しても作成した子プロセスだけを終了します。既存のプレイ中プロセスを検索して停止しません。
失敗したworld・ログは削除せず残します。

### 保存先の隔離

各runで `HORIZON_HOME`、`HORIZON_LAUNCHER_HOME`、`XDG_DATA_HOME`、`XDG_CONFIG_HOME` とプロセスのcwdを分離します。
`HORIZON_LAUNCHER_NO_SELF_UPDATE=1`、`HORIZON_NO_AUDIO=1`、`HORIZON_NO_DISCORD=1` とテスト用telemetry設定も指定します。
**cwdも分離することが重要です。** 実行した版ではrun直下にも `saves/` やcacheができました。
endpointはrun固有の `control.json` を使い、通常プレイの制御endpointを再利用しません。
endpoint内には認証tokenがあるため、内容をレポートへ含めません。

## 自動化できる範囲

| 層 | 使用するもの | 判定できるもの |
| --- | --- | --- |
| Modの計算コア | `cargo test` と原作由来fixture | stats、trait合成、修理、broken、データ変換 |
| Wasm契約 | `cargo horizon-mod test / check`、今後追加するserver harness | 権限・API互換・fuel・trap・応答 |
| 本体の保存・通信 | Rustのunit / integration / parity harness | component保持、packet往復、slot・drop・world保存 |
| ゲーム操作 | `horizon-ctl` / JSON / MCP | state、block、inventory、components、menu、採掘・使用・攻撃、save / quit |
| 見た目 | ctl screenshotと固定camera・画像比較 | toolの部品・色、broken外観、液面・鋳造表示 |
| 複数プレイヤー | 別home・別endpointの複数client＋専用server | slot競合、同期、接続・切断、Mod不一致 |
| 性能 | ctl bench / stats、本体のtick計測 | frame time、tick time、機械数に対する変化 |
| launcher | CLIのversions / prepare / play、MCP | 準備・選択・起動。ゲームロジック自体はctlで検証 |

Windowsの制御transportは、localhost TCPとtokenを記録したJSON endpointです。Unix domain socketがなくてもctlが対応します。
画面の座標をクリックする自動操作を検証の中心にする必要はありません。
なお、`--offscreen` は今回のWindowsログインセッションで動きました。Windowsサービスや未ログイン状態でも必ず動くことは未確認です。

参照: [control transport](A:/Project/horizon/crates/horizon-client/src/control.rs)、[ctl一覧](A:/Project/horizon/docs/horizon-ctl-commands.md)、[launcher一覧](A:/Project/horizon/docs/launcher-commands.md)。

## TConstructの検証へどうつなげるか

1. **pure Rust:** 木材・flintの部品構成を固定したfixtureでstats・trait・修理境界を確認する。
2. **host / sandbox:** server APIのrole・tick回数・mutation transactionをゲームなしで検証する。
3. **ゲーム内tool:** テスト用toolを作り、inventoryのcomponentsとdamage、採掘結果、broken後の個数、修理後の値をassertする。
4. **作業台:** open / menu / click-slot等で部品作成・組み立て・修理を操作し、入力・出力・carried stackの総量を確認する。
5. **永続化:** 保存・終了・再起動、drop / pickup、収納、dimension移動、chunk unloadを追加する。
6. **複数人:** それぞれの独立endpointから同じ台を操作し、revision競合と重複消費・複製がないことを確認する。
7. **機械・描画:** tickを固定して流体・燃料・coolingの収支を確認し、固定視点で描画を比較する。

既存の `stack_value()` は `damage` と保存componentのSNBTをJSON応答へ出すため、内部状態を確認する土台があります。
今後のtool stateがそこへ載れば、見た目だけで判断せず状態までassertできます。
一方、Mod固有のstats、trait一覧、tank温度、transaction revision等を直接確認するAPIはまだありません。
必要に応じてテスト用fixture・診断queryを追加し、プレイヤー向けの無制限な内部書き換えAPIにはしません。

`/tick freeze` / `/tick step` や状態待ちを活用する設計が候補ですが、今回のsmokeでは未使用です。
特にfreeze中に「Nゲームtick進むまで待つ」命令を組み合わせると停止するため、待つ条件と進める側を明確に分けます。

参照: [stackの観測値](A:/Project/horizon/crates/horizon-client/src/game/agent.rs:349)、[詳細実装計画](tconstruct-implementation-plan.md)。

## 既存の上流スクリプトに必要なWindows対応

`tools/agent_control_e2e.py` と `tools/lan_e2e.py` は良い試験素材ですが、現状のままWindowsで全suiteが動くとは扱えません。

- `agent_control_e2e` はmacOS以外で `vscreen` を呼ぶため、Windowsの起動分岐が必要。
- バイナリを `horizon-ctl` / `horizon-client` として組み立てる箇所を `.exe` に対応させる。
- scratch内へresourceのsymlinkを作る処理がある。Windowsのsymlink権限に依存しない `--resources` 指定等へ対応する。
- 現在はcheckoutの `.local/horizon-client/<version>` を前提にするため、resource引数を追加するか再生成する。
- readiness、socket、プロセス終了・タイムアウトをWindowsでも共通化し、今回の専用smokeから段階的に統合する。

`lan_e2e` の既存scenario一覧はmenu・機械・戦闘・同期の設計例として再利用できます。
今回の調査では既存suiteを変更せず、小さいWindows向けprobeで実行可能性を実証しました。

参照: [上流E2E](A:/Project/horizon/tools/agent_control_e2e.py)、[LAN E2E](A:/Project/horizon/tools/lan_e2e.py)、[隔離環境helper](A:/Project/horizon/tools/lab.py:73)。

## 残る準備と未検証事項

- フォークのclient・ctlを同じコミットから `--features mods` でビルドする。Modなしの実行成功はMod runtimeの検証ではない。
- `wasm32-unknown-unknown` の導入と、Mod側のSDK / CLIの版を揃える。
- Python shimを避け、本体ツール用のPython 3.10以上の実体を固定する。
- フォークのexporterに対応するresourceをcheckoutの `.local/` へ生成する。Prismの生のlayoutを公式形式と同一と仮定せず、launcherのsource解決か既存 `linked-minecraft` を使う。今回この再生成は未実施。
- 原作比較には別途Minecraft 1.20.1＋Forge＋対応TConstruct / MantleとJava 17環境を固定する。確認したPrism instanceは26.3のみで、原作oracle環境の存在は確認できていない。
- dedicated / 2-client / LAN、機械tick、Modのreload・trap、画像差分、性能の反復測定は未実施。

これらを段階的に加えれば、変更のたびに「コア → sandbox → 本体 → ゲーム内 → 保存復帰 → 複数人」の順で実行し、失敗時のJSON・ログ・PNGを保存する検証pipelineを構築できます。
