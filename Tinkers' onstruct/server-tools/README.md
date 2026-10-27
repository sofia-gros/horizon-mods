# TConstruct server tool state adapter

独立した補助server packageです。親のコアをclient-adapter featureなしで使います。API 0.3 / fork API level 10、capability `players` と実験capability `items-read` / `items-edit` / `items-tools` が必要です。SDKは検証済み本体 `9928cbea2be1ccfdcd899f73399eefc12a2398d5` に固定し、Cargo.lockを管理します。インストール済みv0.2.66では動きません。

MITライセンスと原作の著作権表示は [LICENSE](LICENSE)。共通コアが使う原作入力・出典・固定コミットは [親のthird_party記録](../third_party/README.md) を参照してください。

```powershell
Set-Location -LiteralPath "A:\Project\horizon-mods\Tinkers' onstruct\server-tools"
cargo test --locked -j 1
& 'A:\Project\horizon\target\debug\cargo-horizon-mod.exe' horizon-mod build
& 'A:\Project\horizon\target\debug\cargo-horizon-mod.exe' horizon-mod test
& 'A:\Project\horizon\target\debug\cargo-horizon-mod.exe' horizon-mod check target/horizon-mod/tconstruct_tools.hmod --json
```

ゲームでは親のclient Modと一緒に読み込みます。親フォルダで `& .\tools\play-item-state.ps1 -Inspect`。手持ちを選んで `/tconstruct_tool inspect` を入力すると、サーバーが読んだ素材・damage・broken・コア基礎statsをJSONで返します。vanilla_damageはMinecraft側の値で、コアdamageと別です。

Creative限定の `/tconstruct_tool test-damage 100` と `/tconstruct_tool test-repair 20` で、選択中の個体のコア状態を更新できます。数量は解決済みの耐久ポイントで、素材やレシピを消費する修理ではありません。更新成功時は `item state updated`、`inspect` でコアのdamageとbrokenを確認します。Survivalからのテスト操作は拒否します。

Creativeで `/tconstruct_tool activate` を実行すると、手持ち個体の素材と破損状態を採掘速度・適正採掘へ反映します。`inspect` の `hooks` は有効化後 `"mining"`、未有効化時 `false` です。破損時は速度0.3・適正採掘なしです。テストdamage/repair後も有効化済みの性能を更新します。成功した採掘では、硬度が0以外のブロックでコア耐久を1消費します（Creative・brokenは除く）。攻撃性能は未実装で、vanilla耐久は消費しません。原作は破損時のドロップ判定で手を空にするため、アイテム条件付きlootには差が残ります。

更新時に選択slotとstack全体を取得時の値と比較します。永続的な個体UIDではありません。vanilla品、状態のない品、不正schemaや別carrierは拒否します。clientとserverを同一IDで二重ロードする構成を避けるため、この補助packageは `tconstruct_tools` という別IDです。

詳細は [M3b検証手順](../../docs/tconstruct-m3b-verification.md)。成果物・ワールド・Minecraft由来リソースはGit管理しません。

基本採掘耐久の実ゲーム試験・残る制限は [M3b4検証記録](../../docs/tconstruct-m3b4-verification.md) を参照してください。
