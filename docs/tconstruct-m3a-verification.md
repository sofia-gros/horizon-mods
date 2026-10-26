# M3a: アイテム状態の保存・通信

M3全体は作業中です。M3aは、後の性能hookで使う個体状態が保存・通信で欠けないための土台です。
本体・Modの開発ブランチは `codex/tconstruct-item-hooks`。M2の `codex/server-mod-game-integration` とは分離しています。

## 対象

- 本体: `A:\Project\horizon`。検証済みソースは `dd8c06ef640adc4c5e57d343f2af55c0ca969986`。M2の `efeb2118` に upstream `749a8a2a` を取り込んだ `0dd64e68` と、汎用stack送信修正からなる。
- Mod: `A:\Project\horizon-mods\Tinkers' onstruct`。SDKは引き続き `88b7bfafc01c7c475548d4379845ff4a9f944dc0` 固定。
- manifest: API 0.1 / client、`command` capability。`/give` は実行プレイヤーの権限に従う。
- resource: ローカルに準備済みのv0.2.66 / Minecraft 26.3。ゲームファイル・成果物・ワールドはGit管理しない。

## 変更

本体の `lan/guest.rs` の共通stack送信処理で、既存の `minecraft:custom_data` をNBT componentとして渡します。受信・保存の既存codecを再利用します。本体へTConstructのschema・素材名・計算処理は入れません。
`item_sync` の回帰試験では、ネストした任意の状態を保存componentへ変換し、実際の `Slot` packet encode/decode と受信adapterを通して比較します。既存のenchantment・name・damage・trimの確認も維持します。

Modに `/tconstruct specimen <head> <handle> <binding> [core-damage]` を追加。コアで素材を評価・damageを上限へ丸め、JSONを `minecraft:custom_data.tconstruct.state_json` へ格納した木のつるはしを作ります。vanillaの省略可能なnamespaceとSNBTの単引用符を使い、現行ホストの256文字制限内に収めます。未知素材・不正な整数・余分な引数を拒否します。

`broken=true` はコア状態の保存試験です。ゲーム内の採掘速度・攻撃・耐久消費・破損時消失・修理はすべてvanillaのままです。実際のTConstructツールとして使える段階ではありません。

## 再ビルドと自動検証

メモリ不足で中断したため、重いビルド・試験・ゲームは同時に動かしません。各コマンドの成功を確認して次へ進みます。Cドライブの空き不足も確認したため、一時ファイルをAドライブの専用フォルダへ分離します（システム設定は変更しません）。

```powershell
$env:CARGO_BUILD_JOBS = '1'
$env:RUST_MIN_STACK = '16777216'
$temporaryBuild = 'A:\Project\horizon-mods\.local\tmp'
New-Item -ItemType Directory -Path $temporaryBuild -Force | Out-Null
$env:TEMP = $temporaryBuild
$env:TMP = $temporaryBuild
Set-Location -LiteralPath 'A:\Project\horizon'
cargo test --locked --no-default-features -p horizon-client --lib item_sync::tests -j 1
cargo build --locked --profile fast --config 'profile.fast.package.horizon-client.opt-level=1' --no-default-features --features mods -p horizon-client --bin horizon-client --bin horizon-ctl -j 1
Set-Location -LiteralPath "A:\Project\horizon-mods\Tinkers' onstruct"
cargo test --locked -j 1
& 'A:\Project\horizon\target\debug\cargo-horizon-mod.exe' horizon-mod build
& 'A:\Project\horizon\target\debug\cargo-horizon-mod.exe' horizon-mod test
& 'A:\Project\horizon\target\debug\cargo-horizon-mod.exe' horizon-mod check target/horizon-mod/tconstruct.hmod --json
Set-Location -LiteralPath 'A:\Project\horizon-mods'
& 'A:\miniconda3\python.exe' -B tools/probe_horizon_item_state.py
& 'A:\miniconda3\python.exe' -B tools/probe_horizon_item_state.py --in-frame
```

実ゲーム試験は一度に一つのゲームだけ起動し、runごとにcwd・home・一時ファイル・control endpointを分離します。3個体の素材・damage・brokenをクライアントinventoryから取り出して期待値と比較し、保存・終了・再起動後にも同じ値か確認します。vanilla diamond 3個の保持、未知素材を拒否してアイテムが増えないことも確認します。結果とinventoryは `.local/item-state-tests/<run>/` に残ります。

## 手動確認

```powershell
Set-Location -LiteralPath "A:\Project\horizon-mods\Tinkers' onstruct"
& .\tools\play-item-state.ps1
```

Tでチャットを開き、次を一つずつ入力します。

| 入力 | 期待結果 |
| --- | --- |
| `/tconstruct specimen wood wood wood` | `TConstruct state specimen` という木のつるはし1本。内部状態はwood/wood/wood、damage=0、broken=false |
| `/tconstruct specimen flint wood wood 17` | 同名の木のつるはし1本追加。内部状態はflint/wood/wood、damage=17、broken=false |
| `/tconstruct specimen flint wood wood 85` | 同名の木のつるはし1本追加。内部状態はflint/wood/wood、damage=85、broken=true |
| `/tconstruct specimen stone wood wood` | `unsupported material tconstruct:stone`。本数が増えない |

Escから保存して終了し、次で同じ専用ワールドを再開します。

```powershell
& .\tools\play-item-state.ps1 -ResumeLatest
```

3本が残っていることを確認します。内部値を確認する場合、起動時に表示されたテストフォルダのendpointに対して、別のPowerShellから読み出せます。

```powershell
$testRun = '起動時に表示されたテストフォルダの絶対パス'
& 'A:\Project\horizon\target\fast\horizon-ctl.exe' --socket "$testRun\control.json" inventory
```

各slotの `components` 内の `state_json` に上表の値があります。control endpointはローカル操作用tokenを含むためコミットしません。`-ValidateOnly` はパス確認、`-InFrame` は代替実行経路です。再開時は初回と同じ実行経路を指定してください。

## 残るM3の仕事

現行server WITはlog・storage・playersであり、個体の取得・改変、採掘性能、攻撃、耐久介入の契約はありません。次は汎用APIの最小単位を設計し、サーバー側で個体を識別・検証して性能と耐久を適用します。別個体の混同、vanilla回帰、壊れた個体の非消失、権限、保存・同期を通して初めてM3完了です。

M3aの実ゲーム試験は統合サーバーとin-frameを対象とします。LAN / dedicatedでの個体状態、drop・container経由、creative画面で編集したstackの実操作は未検証です。保存codecと送受信adapter・packet codecの単体往復に成功しても、これら全経路を検証済みとは扱いません。

## 検証記録

- 本体の保存・実packet往復テスト: 1件成功。
- Mod: native 13件、sandbox 7ステップ成功、成果物check成功。`command` importのみでAPI level 0。
- 修正前の本体M2 `efeb2118`（SHA-256 `46534b7c085461e61da6726ca4f79770fb80ecf85ac131d2e2ded3570f490ebe`）で `.local/item-state-tests/integrated-ymou2uvy/` を実行。3本の名前は保持されるがcustom dataがすべて欠けるため、試験は期待どおり失敗。生成コマンド自体の成功はgame.logで確認。
- Mod成果物: SHA-256 `c801f92f04ae88da0d76b0db44af8e69e96059d90a5fd27b3bca11dd74ed270f`、88,544 bytes。
- 修正後の本体成果物: SHA-256 `362f8d0c19954635446aee480e7645cbba17ffa94d7a2036b3aba8119ad4ba46`。上記のfast profile / client opt-level=1 / jobs=1でビルド。メモリ不足の再発なし。
- 実ゲーム: 統合サーバー `.local/item-state-tests/integrated-fqcibo60/report.json` 6項目、in-frame `.local/item-state-tests/in-frame-0e94tami/report.json` 6項目、計12項目すべて成功。3個体は生成直後と保存・再起動後に完全一致。通常のdiamondも保持。
- 手動起動スクリプトはPowerShell構文検査と `-ValidateOnly` に成功。2026-10-26、ユーザーから保存・再起動後も試験アイテム3本が残ったとの確認を受領。添付画像でも3本と `TConstruct state specimen` の表示を確認。内部JSON各値の一致は上記自動試験の結果であり、今回の手動報告で個別確認されたとは扱わない。
- M2回帰: 新しい本体で統合サーバーprobeの14項目が成功。起動回数・tick重複なし・旧client診断・chat・破壊/設置・Nether移動・pause/resumeを確認。`.local/server-mods-tests/integrated-3nh7jptz/report.json`。
- 静的チェック: 本体clientのlib / bins（mods有効）とMod全targetのClippyをwarnings deniedで実行して成功。両リポジトリのrustfmt、本体の公開ツリー・履歴検査にも成功。
