# M3b3: 個体ごとの採掘性能

M3b3の最初の区切りはHorizonの汎用 `minecraft:tool` componentの保存・通信・採掘計算です。この節は本体 `2e7ec5b6` 時点の記録です。後続のTConstruct個体への適用は末尾を参照してください。

この土台の本体コミットは `2e7ec5b6`、ブランチは `codex/tconstruct-item-hooks`。Mod側のRustコード・成果物はM3b2から変更せず、検証driverと手順を追加しています。

## 汎用の採掘経路

個体のTool設定をサーバーから送信し、Slotの実際の通信codecを通してクライアント側でも復元します。速度と採掘可否は独立した最初の一致ruleに従い、一致しなければToolのdefault speed / falseになります。タグ指定と直接ブロック列挙の両方を扱います。設定のない個体は既存のprototypeを使います。ルールなしToolのdefault speedを1固定にしていた経路も修正しました。

Minecraft 26.3の `Tool.getMiningSpeed` / `isCorrectForDrops` のバイトコードを確認し、Java oracleが実際の `Tool.CODEC` と `ItemStack.getDestroySpeed` / `isCorrectToolForDrops` を使って比較します。TConstruct由来の計算や素材IDは本体へ入れていません。

## 自動確認

`tools/probe_horizon_tool_components.py` と `--in-frame` を順に実行します。Modを読み込まない新規テストワールドで、同じ木のつるはしcarrierに異なるTool設定を付けます。

- 統合サーバー `.local/tool-component-tests/integrated-z1xr697z/report.json` 16項目成功。
- in-frame `.local/tool-component-tests/in-frame-4elseamr/report.json` 16項目成功。
- 両構成とも採掘所要tickは、speed=90が1、2が23、3.5が18、空ruleでdefault=0.5が306。後者はharvest=falseでもあるため、単純な速度比の比較ではありません。実際の破壊・ドロップ可否・保存復帰を別々に確認しています。これはゲーム全体のフレーム速度測定ではありません。
- Java比較 `A:\Project\horizon\.local\rust-tool-component-parity\report.json` 144ケース一致。直接ブロック指定・rule順序・速度/採掘可否の独立評価・空rule・prototypeへのfallbackが対象。タグの往復と評価はnative試験で確認。
- 新規のitem_tool試験2件、item_sync試験3件、既存Tooltipsを含む該当試験11件（負荷測定1件ignored）、protocol item試験11件成功。client lib/bins / probeのClippyも成功。
- 追加回帰はItems / mod_itemsの10件、protocol library全100件成功。wire比較 `.local/item-wire-parity/report.json` は374個体・全122種類のcomponentを確認し、Toolだけをdecodeした個体は再encode時のbytes一致も必須にしています。
- 旧本体 `c0993fa5` を使った対照試験 `.local/tool-component-tests/in-frame-awqey0wh/report.json` は、既存のprototype採掘可否を使ってしまう `default: harvest follows the stack rule` で意図どおり失敗。起動失敗などを差分検出として扱っていません。

最初の本体成果物SHA-256: `cdfc0984173fe6aab33f2c9d9f200f28386d30874151dddf2988f027b9612769`。同じビルドでM3b2の統合サーバー `.local/item-state-tests/server-integrated-4wrhjmf7/report.json` とin-frame `.local/item-state-tests/server-in-frame-trz2y0a7/report.json` の計60項目も成功しました。

受信時の負数・最小整数・切り詰められたholder setを拒否する試験を追加し、protocol library全101件が成功。最終ビルドのSHA-256は `58c150b58748c469bd4c78f70534a914c98e245ddb4f55ac749a0c55a3bfda76` です。API契約はlevel 5から変更なし。既存のM3b2 server packageのSDK固定先は `c0993fa5` のままです。

最終ビルドでも統合サーバー `.local/tool-component-tests/integrated-bbtusha8/report.json` とin-frame `.local/tool-component-tests/in-frame-sflnnoek/report.json` の計32項目が成功し、上記tick値も同じでした。wireの374個体・122種類も再確認済み。Mod無効client check、rustfmt、公開ツリー・履歴検査も成功。Python harnessは確認済みのPython 3.13を使って実行できます。既定のoracle buildではなく、この端末ではメモリ節約のため `fast` / client opt-level=0 / jobs=1の同じprobeを事前ビルドして使用しました。

本体と既存のclient / server packageの組み合わせは `.local/artifacts/m3b3a-2e7ec5b6/` に保存しています。最終fetchは上流 `4d6875ec`。前回確認以降に関連Mod・SDK・runtime・launcherのマージはありませんでした。

## 手動操作

従来と同じ分離テストワールドを起動できます。

```powershell
Set-Location -LiteralPath "A:\Project\horizon-mods\Tinkers' onstruct"
$artifact = 'A:\Project\horizon-mods\.local\artifacts\m3b3a-2e7ec5b6'
& .\tools\play-item-state.ps1 -Inspect -Client "$artifact\horizon-client.exe" -ClientMod "$artifact\tconstruct.hmod" -ServerMod "$artifact\tconstruct_tools.hmod"
```

新規テストワールドで以下を入力します。専用ワールドでも手持ちを置換するので、試験用slotを選んで使います。

```text
/gamemode creative
/item replace entity @s weapon.mainhand with minecraft:wooden_pickaxe[minecraft:tool={rules:[{blocks:"minecraft:stone",speed:90f,correct_for_drops:true}]}]
/setblock ~ ~-1 ~ minecraft:stone
/setblock ~ ~ ~2 minecraft:stone
/gamemode survival
```

設置した石を掘ると通常の木のつるはしより速く破壊し、丸石が出ます。Creativeへ戻し、`speed:90f` を `speed:2f` にして同じ位置へ石を設置すると遅くなります。保存後、上記コマンドに `-ResumeLatest` を加えて再開してもTool設定が残ります。`-InFrame` でも同じ操作ができます。

これは汎用Toolの動作確認です。コアのdamage/brokenから性能を変更する機能、独自の耐久方針、修理レシピ、Tool componentの削除patchはこの区切りでは扱っていません。人によるM3b3の確認は未実施です。

## 後続の設計と原作の確認

サーバーModが解決済みのToolルールと状態文字列を同じ比較付き更新で適用するAPIを追加しました。素材・trait・harvest tierの変換はMod側、本体は有限な速度・ルール数・selector数・文字列サイズと権限を検証します。

原作の固定コミット `f4e77d43` の `MiningSpeedToolHook.getDestroySpeed` と `ToolHarvestLogic` を確認すると、broken時の速度は0.3で、破壊時のドロップ判定は手を空にして行います。採掘不能にするのではなく、まず速度0.3と採掘可否falseを個体設定へ反映し、クライアントとサーバーを揃えます。Toolを実際に空の手へ一時的に置き換える処理まではまだ実装されておらず、アイテム条件を使う特殊なlootでは原作と差が残ります。修理後の復帰・別個体の保持・trap時の取消を確認してから、実際の採掘によるコア耐久消費へ進みます。

## TConstructの採掘性能を試す

本体 `50c7c14650c8f05a245f93f81463da095ef9882c`、server package `tconstruct_tools` 0.2.0、client packageを使用します。以下の保存済み成果物は自動試験に使用したものです。

統合サーバー `integrated-n5ecjarg` とin-frame `in-frame-1hhsznhz` で各13項目、計26項目成功。石の採掘は木製23 tick、火打石製18 tick、破損時の土は56 tick、修理後の石は18 tickでした。原作Javaの採掘速度8ケースとRust側の値も一致しました。これらは分離テストワールドでの自動検証で、ユーザーによる実機確認はまだです。

```powershell
Set-Location -LiteralPath "A:\Project\horizon-mods\Tinkers' onstruct"
$artifact = 'A:\Project\horizon-mods\.local\artifacts\m3b3c-50c7c146'
& .\tools\play-item-state.ps1 -Inspect -Client "$artifact\horizon-client.exe" -ClientMod "$artifact\tconstruct.hmod" -ServerMod "$artifact\tconstruct_tools.hmod"
```

新規ワールドで `/gamemode creative` とし、次を入力します。

```text
/tconstruct specimen wood wood wood 0
/tconstruct specimen flint wood wood 17
/tconstruct specimen flint wood wood 85
```

各つるはしを手に持って `/tconstruct_tool activate` を実行し、`item state updated` を確認します。`/tconstruct_tool inspect` は各個体の状態と `"hooks":"mining"` を表示します。Survivalに変更して石を掘ると火打石製は木製より速く、鉄鉱石は火打石製だけが適正採掘できます。damage 85の破損個体は遅く掘り、消えません。

Creativeに戻し破損個体を持って `/tconstruct_tool test-repair 20` とするとdamage 65になり、採掘性能が回復します。`/tconstruct_tool test-damage 20` で再び破損します。保存して終了後、起動コマンドに `-ResumeLatest` を追加すれば同じワールドを再開できます。別の実行方式は `-InFrame` を追加します。

この段階では採掘によってコア耐久が減らず、攻撃性能・通常の素材消費による修理も未実装です。破損時の特殊なloot条件にも原作との差があります。上記のtest-repairは検証用の数値操作です。
