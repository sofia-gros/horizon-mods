# M3b: サーバーからの個体読み取りと性能hook

M3bは作業中です。最初の区切りM3b1では、実際の採掘・戦闘・耐久hookより先に、サーバーが手持ちのどの個体を読んでいるかを固定します。2026-10-27、ユーザーから本体 `235cac33` / Mod `0272914` の実機確認済み・問題なしとの報告を受領しました。添付画像でも3個体の素材・damage・broken・基礎statsが期待値と一致しています。M3b2以降の手動確認とは区別します。
作業ブランチは引き続き本体・Modとも `codex/tconstruct-item-hooks`。

本体とserver SDKの固定コミットは `235cac33f056efaeb316e5e309b767e14498ab79`。本流 `800b9156` を含みます。client SDKは従来の `88b7bfaf` 固定を維持します。検証用本体はfast profile / client opt-level=0 / jobs=1で作成し、一時ファイルをAドライブへ分離します。ゲームの性能比較は行いません。

## M3b1の実装範囲

本体に汎用・読み取り専用の実験capability `items-read` を追加しました。WITの `main-hand(player, custom-path)` は、サーバー側の手持ちitem ID・count・vanilla damageと、指定したcustom-data内の文字列を返します。オフライン / 空手はnone、キー欠落 / 文字列以外はcustom textなしです。
pathは8キー・各128 UTF-8 bytes、文字列は4096 bytes、各hookは16回まで。巨大文字列や不正Unicodeは切り詰めず拒否します。アイテム改変・コマンド実行は公開しません。
データはsnapshot時にコピーして固定します。以前の「共有データ」という説明は訂正します。StackDataはBoxなのでコピーが発生します。M3b2ではコマンド専用Modのtickごとのstackコピーを省き、コマンド実行時に新しいviewを作ります。NBTの全体シリアライズは行いません。

本流の `command.run-quiet` がlevel 3になったため、読み取りAPIは**このforkのlevel 4**です。upstreamでlevel 4が同じ機能を保証するという意味ではありません。SDKはこのAPIを検証したforkのGitコミットへ固定します。

Mod側の `Tinkers' onstruct/server-tools/` は独立した補助server packageです。IDは `tconstruct_tools`、API 0.3、capability `players`（actor型）/ `items-read` と、`items-read` のexperimental opt-inを使います。client診断Mod `tconstruct` と別IDで読み込みます。共通コアは親Cargo packageを `default-features = false` で参照し、client用Wasm exportをserver componentへ混ぜません。

`/tconstruct_tool inspect [request]` はサーバーが実際に持つ `minecraft:custom_data.tconstruct.state_json` を読み、コアで厳密に検証して素材・damage・broken・基礎statsを返します。requestは応答の取り違えを防ぐ識別子です。vanilla damageとコアdamageを分け、`hooks=false` を返します。

**この区切りでもゲーム内性能・耐久消費・破損時の挙動はvanillaです。** 表示するmining / attack / speedはコアの基礎計算値です。jagged等の条件付き効果やbrokenフラグをゲームへ適用した結果ではありません。

## 手動操作

ビルド・自動検証の完了後、次で分離した専用ワールドを開きます。

```powershell
Set-Location -LiteralPath "A:\Project\horizon-mods\Tinkers' onstruct"
& .\tools\play-item-state.ps1 -Inspect
```

3本を作ります。

```text
/tconstruct specimen wood wood wood
/tconstruct specimen flint wood wood 17
/tconstruct specimen flint wood wood 85
```

ホットバーの1〜3を選び、その都度 `/tconstruct_tool inspect` を入力します。
上記は標準の統合サーバーでの手順です。`-InFrame` では初期ホットバーが建材で埋まるため、専用テストワールドで生成前に `/clear @s` を実行するか、生成した3本をホットバーへ移動してから選んでください。自動試験は新規の専用ワールド内だけで初期インベントリを空にし、3本の生成完了後にvanilla品を追加して選択順を固定します。

| 選ぶ個体 | parts | damage | broken | durability | mining | attack | speed |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 1本目 | wood / wood / wood | 0 | false | 60 | 2.0 | 1.0 | 1.2 |
| 2本目 | flint / wood / wood | 17 | false | 85 | 3.5 | 2.25 | 1.2 |
| 3本目 | flint / wood / wood | 85 | true | 85 | 3.5 | 2.25 | 1.2 |

生成直後のvanilla_damageは0、hooksはfalseです。保存して終了し、`& .\tools\play-item-state.ps1 -Inspect -ResumeLatest` で同じワールドを開いて再確認します。`-Inspect` なしのM3aワールドとは分離しています。

## 自動検証

`tools/probe_horizon_item_state.py --server-mod "Tinkers' onstruct/server-tools/target/horizon-mod/tconstruct_tools.hmod"` で、M3aの6項目にサーバーからの3個体読み取り（保存前・再起動後）とvanillaアイテムの拒否を追加します。`--in-frame` でも同じ試験を行います。
結果は `.local/item-state-tests/server-<mode>-<run>/report.json`。各応答のrequestを照合し、古いチャット返信を成功として使いません。ゲームは順に起動します。

本体のビルドコマンド:

```powershell
Set-Location -LiteralPath 'A:\Project\horizon'
$env:CARGO_BUILD_JOBS = '1'
$env:RUST_MIN_STACK = '16777216'
$temporaryBuild = 'A:\Project\horizon-mods\.local\tmp'
New-Item -ItemType Directory -Path $temporaryBuild -Force | Out-Null
$env:TEMP = $temporaryBuild
$env:TMP = $temporaryBuild
cargo build --locked --profile fast --config 'profile.fast.package.horizon-client.opt-level=0' --no-default-features --features mods -p horizon-client --bin horizon-client --bin horizon-ctl -j 1
# Windowsのopt-level=0検証用バイナリはメインスレッドのstackも拡大する。
cargo rustc --locked --profile fast --config 'profile.fast.package.horizon-client.opt-level=0' --no-default-features --features mods -p horizon-client --bin horizon-client -j 1 -- -C link-arg=/STACK:16777216
```

Modは親フォルダと `server-tools/` でそれぞれ `cargo test --locked`、`cargo horizon-mod build / test / check` を実行します。server成果物は `server-tools/target/horizon-mod/tconstruct_tools.hmod`、client成果物は従来の `target/horizon-mod/tconstruct.hmod` です。

## 次の区切り

次は個体の取得・変更の所有権とrevision、採掘のserver判定とclient予測、攻撃、耐久消費、broken時の非消失を汎用契約として追加します。read-only APIだけでM3全体を完了とは扱いません。LAN / dedicatedや複数人の読み取り・改変も、実施した範囲だけを別途記録します。

## M3b2: 比較付き状態更新（2026-10-27）

本体 `c0993fa55d0e6d81e384fb35d8a4eda2fb18dbd9` とserver package 0.1.1を使用します。本体・Modともブランチは `codex/tconstruct-item-hooks`。実験API `items-edit`（fork level 5）は既存custom-data文字列だけを置換し、適用直前に選択slotとstack全体を取得時の値と比較します。数量・item ID・vanilla damage・別タグを変更しません。同じ値のstackを識別する永続UIDや、全inventoryのrevisionではありません。hookのtrapやcommandエラー時には未確定の変更を破棄します。

起動は上記と同じ `& .\tools\play-item-state.ps1 -Inspect`。3本を生成後、2本目（flint / wood / wood、damage=17）を選択して以下を順に実行し、毎回 `inspect` で確認します。

| 操作 | 期待damage | 期待broken |
| --- | --- | --- |
| `/tconstruct_tool test-damage 4294967295` | 85 | true |
| `/tconstruct_tool test-repair 20` | 65 | false |
| `/tconstruct_tool test-repair 4294967295` | 0 | false |
| `/tconstruct_tool test-damage 20` | 20 | false |

1本目・3本目の状態は変わらず、vanilla_damage=0、hooks=falseです。`/gamemode survival` 後の `test-repair 5` は `test edits require creative mode` で拒否します。`/gamemode creative` へ戻して保存終了し、`-Inspect -ResumeLatest` で2本目がdamage=20のまま残ることを確認できます。実際に採掘・攻撃したときの性能や耐久消費、素材を消費する修理はまだ接続していません。人によるGUI確認は未実施です。

自動検証は `tools/probe_horizon_item_state.py --state-edits --server-mod "Tinkers' onstruct/server-tools/target/horizon-mod/tconstruct_tools.hmod"`。`--in-frame` も同様です。

- 統合サーバー `.local/item-state-tests/server-integrated-tmm993q0/report.json` 30項目、in-frame `.local/item-state-tests/server-in-frame-y55bnrz4/report.json` 30項目、旧SDKのM2 `.local/server-mods-tests/integrated-aws6z50c/report.json` 14項目が成功。
- 本体Mod library 35件、API reference 4件、sandbox 29件成功（負荷測定1件ignored）。client libraryの取得・比較更新試験2件、client lib/bins・Mod全targetのClippy、Mod無効client check、SDK/welcomeのWasm Clippy、rustfmt、公開ツリーと履歴検査成功。
- server Mod native 3件、sandbox 3ステップ、成果物check成功。client成果物はM3b1から変更なし。
- 本体SHA-256 `171c622295a11f5f63c1d3fb2fb13b9d5f5072606e99bccf0e2e473cc3f2a0fe`。server成果物SHA-256 `c8625f3989c24f6286f05be713b0954ffb2cf15452e0dfe7e2decee45b85dbbf`。組み合わせは `.local/artifacts/m3b2-c0993fa5/` に保存。
- 上流は `72e678be` まで確認。前回確認済み `97ddd2c6` 以降に関連Mod・SDK・runtime・launcher変更はなし。コマンド専用のitem読み書きModについて、通常tickのBox入りstack全コピーを省く処理も含む。

次はM3b3で採掘のclient予測・server判定を接続します。就寝中も進行する指示により、手動テストを待たず着手します。

## M3b1検証結果（2026-10-27）

- 本体: Mod library 34件、API reference 4件、sandbox 29件成功（負荷測定1件ignored）。凍結したitem view・UTF-8サイズ上限・不正Unicode拒否のclient library試験1件も成功。
- Clippy: client lib/binsのmods有効構成、horizon-mods全target、SDKとwelcomeのWasm構成がwarnings deniedで成功。Mod無効のclient check、rustfmt、公開ツリーと履歴検査も成功。
- client Mod: native 13件、sandbox 7ステップ、成果物check成功。server Mod: native 2件、sandbox 2ステップ、成果物check成功。双方のClippy成功。
- 統合サーバー: `.local/item-state-tests/server-integrated-8xzncopd/report.json` の13項目成功。
- in-frame: `.local/item-state-tests/server-in-frame-far74vqi/report.json` の13項目成功。初期ホットバーの建材を対象品と誤って選んだ初回試験は失敗として残し、新規テスト内の初期化・生成完了待ちを揃えて再試験した。
- 旧SDKのM2 probe: `.local/server-mods-tests/integrated-5_vbt26h/report.json` の14項目成功。join/tick、chat、破壊・設置、Nether移動、pause/resumeを確認。
- 本体成果物SHA-256: `661c0bd28736e39064b3cc00e056d5d8dc96185639a97131504c5f51d43f57b4`。Windows PEのメインstack reserveは16 MiB、補助threadもRUST_MIN_STACK=16 MiBで検証。最初のopt-level=0 / 標準stack版は起動時stack overflowで失敗し、上記のリンク設定で再試験して解消した。
- client成果物SHA-256: `04daaa12b0476598e27401ec48dfd7d386f7e594f23e081fcea7de390fde890d`（88,551 bytes）。server成果物: `2f5cd970c6cd5de7d788524b37f734b43b8d7f54247d0dfe76237d651d0cb6a6`（92,741 bytes）。
- 手動起動scriptのPowerShell構文・`-Inspect -ValidateOnly`、Python構文検査成功。ユーザー自身によるM3b1のGUI操作もその後確認済み・問題なしと報告受領。LAN / dedicatedの読み取りと、実際のゲーム性能・耐久hookは今回未検証・未実装。
