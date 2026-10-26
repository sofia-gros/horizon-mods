# TConstruct server tool state adapter

独立した補助server packageです。親のコアをclient-adapter featureなしで使います。API 0.3 / fork API level 5、capability `players`（呼び出し元player型）と実験capability `items-read` / `items-edit` が必要です。SDKは検証済み本体 `c0993fa55d0e6d81e384fb35d8a4eda2fb18dbd9` に固定し、Cargo.lockを管理します。インストール済みv0.2.66では動きません。

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

**実際の採掘性能・攻撃・vanilla耐久処理は変更しません。** `hooks=false` の段階です。更新時に選択slotとstack全体を取得時の値と比較し、既存の状態文字列だけを書き換えます。永続的な個体UIDではありません。vanilla品、状態のない品、不正schemaや別carrierは拒否します。clientとserverを同一IDで二重ロードする構成を避けるため、この補助packageは `tconstruct_tools` という別IDです。

詳細は [M3b検証手順](../../docs/tconstruct-m3b-verification.md)。成果物・ワールド・Minecraft由来リソースはGit管理しません。
