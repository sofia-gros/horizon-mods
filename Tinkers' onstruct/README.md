# Tinkers’ Construct for Horizon

Horizon向け移植の開発場所です。現在は調査・設計段階で、実行可能なModはまだありません。

- 本体: `A:\Project\horizon`（`sofia-gros/horizon` フォーク）
- Mod ID / namespace案: `tconstruct`
- Cargo package案: `tconstruct-horizon`
- 対象原作: Minecraft 1.20.1系列のTinkers’ Construct 3
- 初期目標: 少数素材から部品・つるはしを作り、採掘・破損・修理する

[詳細実装調査](../docs/tconstruct-implementation-plan.md) に、本体側の追加箇所、Mod構成、API契約案、作業順序、検証条件をまとめています。
[先行調査](../docs/porting-tinkers-construct.md) に、固定した原作コミット、依存、MITライセンス、機能構成を記録しています。

フォルダ名はユーザー指定の `Tinkers' onstruct` です。ModのIDや自動生成されるpackage名とは分離して扱います。
このフォルダでPowerShellコマンドを実行する場合は、たとえば次のようにパスを引用します。

```powershell
Set-Location -LiteralPath "A:\Project\horizon-mods\Tinkers' onstruct"
```

原作コード・アセットの取り込み時にはMIT本文・著作権表示・出典一覧を追加します。現時点では原作ファイルを取り込んでいません。
