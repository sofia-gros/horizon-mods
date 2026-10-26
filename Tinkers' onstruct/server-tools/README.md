# TConstruct server tool inspector

独立した補助server packageです。親のコアをclient-adapter featureなしで使います。API 0.3 / fork API level 4、capability `players`（呼び出し元player型）と実験capability `items-read` が必要です。SDKは検証済み本体 `235cac33f056efaeb316e5e309b767e14498ab79` に固定し、Cargo.lockを管理します。インストール済みv0.2.66では動きません。

MITライセンスと原作の著作権表示は [LICENSE](LICENSE)。共通コアが使う原作入力・出典・固定コミットは [親のthird_party記録](../third_party/README.md) を参照してください。

```powershell
Set-Location -LiteralPath "A:\Project\horizon-mods\Tinkers' onstruct\server-tools"
cargo test --locked -j 1
& 'A:\Project\horizon\target\debug\cargo-horizon-mod.exe' horizon-mod build
& 'A:\Project\horizon\target\debug\cargo-horizon-mod.exe' horizon-mod test
& 'A:\Project\horizon\target\debug\cargo-horizon-mod.exe' horizon-mod check target/horizon-mod/tconstruct_tools.hmod --json
```

ゲームでは親のclient Modと一緒に読み込みます。親フォルダで `& .\tools\play-item-state.ps1 -Inspect`。手持ちを選んで `/tconstruct_tool inspect` を入力すると、サーバーが読んだ素材・damage・broken・コア基礎statsをJSONで返します。vanilla_damageはMinecraft側の値で、コアdamageと別です。

**ゲーム性能・耐久・修理を変更しません。** `hooks=false` の診断段階です。取得元はサーバーのmain handで、inventory改変は行いません。vanilla品、状態のない品、不正schemaや別carrierは拒否します。clientとserverを同一IDで二重ロードする構成を避けるため、この補助packageは `tconstruct_tools` という別IDです。

詳細は [M3b検証手順](../../docs/tconstruct-m3b-verification.md)。成果物・ワールド・Minecraft由来リソースはGit管理しません。
