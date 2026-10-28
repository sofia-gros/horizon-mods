# TConstruct server tool state adapter

独立した補助server packageです。親のコアをclient-adapter featureなしで使います。API 0.3 / fork API level 18、capability `players` と実験capability `items-read` / `items-edit` / `items-tools` / `inventory-transactions` / `workstations` が必要です。SDKは検証済み本体 `0e0facb8eb838c021b6a7a257f3a7fee44b449c6` に固定し、Cargo.lockを管理します。インストール済みv0.2.66では動きません。

MITライセンスと原作の著作権表示は [LICENSE](LICENSE)。共通コアが使う原作入力・出典・固定コミットは [親のthird_party記録](../third_party/README.md) を参照してください。

```powershell
Set-Location -LiteralPath "A:\Project\horizon-mods\Tinkers' onstruct\server-tools"
cargo test --locked -j 1
& 'A:\Project\horizon\target\debug\cargo-horizon-mod.exe' horizon-mod build
& 'A:\Project\horizon\target\debug\cargo-horizon-mod.exe' horizon-mod test
& 'A:\Project\horizon\target\debug\cargo-horizon-mod.exe' horizon-mod check target/horizon-mod/tconstruct_tools.hmod --json
```

ゲームでは親のclient Modと一緒に読み込みます。親フォルダで `& .\tools\play-item-state.ps1 -Inspect`。手持ちを選んで `/tconstruct_tool inspect` を入力すると、サーバーが読んだ素材・damage・broken・コア基礎statsをJSONで返します。有効化後は `vanilla_damage` をコア damage に同期し、個体の最大耐久も標準 component に反映します。

Creative限定の `/tconstruct_tool test-damage 100` と `/tconstruct_tool test-repair 20` で、選択中の個体のコア状態を更新できます。数量は解決済みの耐久ポイントで、素材やレシピを消費する修理ではありません。更新成功時は `item state updated`、`inspect` でコアのdamageとbrokenを確認します。Survivalからのテスト操作は拒否します。

Creativeで `/tconstruct_tool activate` を実行すると、手持ち個体の素材と破損状態を採掘速度・適正採掘へ反映します。`inspect` の `hooks` は有効化後 `"mining"`、未有効化時 `false` です。火打石の jagged は適正採掘の速度を消耗に応じて変え、採掘とテスト修理のたびに反映します。inspect の mining は条件付き計算前の基礎値です。破損時は速度0.3・適正採掘なしです。テストdamage/repair後も有効化済みの性能を更新します。成功した採掘では、硬度が0以外のブロックでコア耐久を1消費します（Creative・brokenは除く）。攻撃の modifier や対象による特殊効果は未実装です。vanilla の自動消費ではなく、コアの結果を標準耐久へ同期します。原作は破損時のドロップ判定で手を空にするため、アイテム条件付きlootには差が残ります。

更新時に選択slotとstack全体を取得時の値と比較します。永続的な個体UIDではありません。vanilla品、状態のない品、不正schemaや別carrierは拒否します。clientとserverを同一IDで二重ロードする構成を避けるため、この補助packageは `tconstruct_tools` という別IDです。

詳細は [M3b検証手順](../../docs/tconstruct-m3b-verification.md)。成果物・ワールド・Minecraft由来リソースはGit管理しません。

基本採掘耐久の実ゲーム試験・残る制限は [M3b4検証記録](../../docs/tconstruct-m3b4-verification.md) を参照してください。

直接 Mob 攻撃の基本耐久消費、素材・jagged による攻撃力、攻撃速度、broken の攻撃禁止は接続済みです。PvP の基本耐久消費も接続済みです。他の modifier や特殊攻撃は未対応です。[近接攻撃の記録](../../docs/tconstruct-melee-design.md)。

## 火打石による試験用ツールの修理

本体 `0e0facb8eb838c021b6a7a257f3a7fee44b449c6` とこの Mod を使います。Creativeで `/tconstruct specimen flint wood wood 85` を作成し、手持ちで `/tconstruct_tool activate` を実行します。別の inventory slot（0〜35）に火打石を置き、Survivalで `/tconstruct_tool repair-flint 1` のようにその slot 番号を指定します（ホットバーは左から 0〜8）。成功時は必要な火打石だけを消費し、耐久・破損・採掘と攻撃の性能を更新します。フリント head の素材レシピのみ対象で、このコマンドはフリント専用です。Station UI の wood / flint 修理と制作は別途接続しています。これは登録済み TConstruct の道具ではなく既存の試験用 carrier に対する操作です。

サーバーはコマンド実行者の同一スナップショットから手持ちと材料を取得し、適用前に両方の完全な stack を照合します。別の inventory 操作で変わった場合、材料・道具とも更新しません。開いている container やカーソル上の持ち物がある場合も拒否します。原作の Station 上の操作や multiplayer における網羅的な検証の代替ではありません。

Station の実装範囲・操作・検証制限は [限定修理](../../docs/tconstruct-station-repair.md) を参照してください。

制作・組立の範囲と操作は [M4 GUI 制作](../../docs/tconstruct-crafting-gui.md) を参照してください。
