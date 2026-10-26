# TConstruct M0・M1検証記録

実施日: 2026-10-26（日本時間）。SDK / CLIは `88b7bfafc01c7c475548d4379845ff4a9f944dc0`。本体作業ツリーのitem-sync関連2ファイルには未検証の後続変更があるが、このCLI・SDK・Modの実装には含めない。

## 成果物

- `Tinkers' onstruct/target/horizon-mod/tconstruct.hmod`、0.1.0、client API 0.1、capabilityなし。
- check成功時85,343 bytes、SHA-256 `285693e5d5e40211b6df200bd84c2a82130f8aab5f0cb4ab85ea83006bdc40c6`。
- ハッシュは今回検証したarchiveを指す。再package時の同一バイト列を保証しない。ソースとlockfileを管理し、成果物・ログ・ワールドはGit対象外。

## 成功した試験

| 試験 | 結果 |
| --- | --- |
| 原作snapshot / MIT / SHA-256 | 選択した27ファイル一致 |
| 依存一覧の再生成比較 | 一致 |
| Java数値oracle再実行 | 23ケース一致。選択した原作メソッドのfloatビット列比較 |
| `cargo test --locked` | コア10件＋oracle fixture比較1件成功 |
| fmt / clippy | fmt成功、全native targetとWasm targetのclippy警告なし |
| Wasm build / package | Gitコミットに固定したSDKで成功 |
| `cargo horizon-mod test` | diagnosticsシナリオ5ステップ成功。再読み込み・未知素材エラーを含む |
| `cargo horizon-mod check` | `ok: true`、コマンド `tconstruct` 登録、importなし |
| ゲーム v0.2.66 / in-frame | 診断表示、save/reopen後の再実行、vanillaブロック・inventory保存を含む22ステップ成功 |

実ゲーム報告: `.local/automation/smoke-zg3_9ao1/report.json`。client SHA-256 `0dff743ea18431c0c49e3c8f0a9c421783309829c789c72d6e84b2ae5d01ac13`、ctl SHA-256 `65fd97e523eb635207865d5e00b59556a90cab032c942ac184ebb5bca8456c33`。v0.2.66同梱の26.3準備済みresourceを使用。

再実行はModをbuildした後、開発場のルートで次を実行する。

```powershell
Set-Location -LiteralPath 'A:\Project\horizon-mods'
& 'A:\miniconda3\python.exe' -B tools/probe_horizon_automation.py --in-frame --dev-mod "A:\Project\horizon-mods\Tinkers' onstruct\target\horizon-mod\tconstruct.hmod" --mod-command 'tconstruct stats wood wood wood' --expect-feedback 'Pickaxe: durability=60, mining=2.000, damage=1.000, speed=1.200; diagnostics only'
```

## 失敗・制限

- v0.2.66 / v0.2.68のintegratedモード: 自動試験のclient診断がunknown command。`command` はserver consoleへ直送、`chat` 経路でもこの組み合わせでは診断できなかった。成功として数えない。失敗報告はそれぞれ `.local/automation/smoke-hqdrgk8j/report.json` と `smoke-3kdbq4wh/report.json`。v0.2.68もresourceはv0.2.66のものを指定した。
- save/reopenは診断Modの再読み込みとvanillaのblock/itemを確認したもの。TConstruct個体のゲーム内保存ではない。
- OracleはForge全体・全modifierの動作を保証しない。破損・修理はnativeコア試験で確認し、ゲーム内hookは未接続。
- 可視ウィンドウ用起動スクリプトはパス・構文検証を実施。自動ゲーム試験はoffscreenで同じclient・resource・Mod・in-frame指定を使用した。

完了前の上流fetchでは `7650451b` まで確認。追加はvanillaの死亡・fluid・shulker・機械/車両menu保存修正で、WIT・SDK・Mod runtime・CLIの追加変更はない。M1の固定SDKは変更しない。menu保存修正はM4着手時の本体追従で再確認する。

ユーザーの起動・操作・期待結果は [Mod README](../Tinkers%27%20onstruct/README.md) に記載。
