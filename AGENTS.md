# この作業場所での開発

- Horizon用の複数Modを扱う。各Modの実体はルート直下の `{mod_name}/` に置く。`mods/` を間に挟まない。
- 共通ガイドは `docs/README.md` を入口とする。Mod固有の仕様・対応Horizon・ビルド方法・制限は各Modの `README.md` に記録する。
- 各Modは独立したプロジェクトとして扱い、Cargoの操作は対象Modのディレクトリで行う。ルートのCargo workspaceは現時点では作らない。
- 新しいAPIを使う前に、対象HorizonのWIT・SDK・ランタイム実装で利用可能か確認する。上流の計画段階の機能を実装済みとして扱わない。
- SDK依存は検証済みのGitコミットに固定し、コードModの `Cargo.lock` を管理する。実験APIの利用はmanifestとModのREADMEに明記する。
- 変更対象のModに適したビルド・検証を実施する。コードModは `cargo horizon-mod test` と成果物の `check`、コンテンツModは成果物の `check` とゲーム内確認を基本とする。未実施の検証は明記する。
- Minecraft由来のゲームファイル、抽出・準備済みリソース、ワールド、認証情報、ビルド成果物をコミットしない。独自に作ったアセットは管理してよい。

## Tinkers’ Construct移植

- 本体は `A:\Project\horizon` のユーザーフォーク `sofia-gros/horizon` を使用する。本体側のAGENTS.md・CONTRIBUTING.mdも読む。
- 今回のMod実体はユーザー指定の `Tinkers' onstruct/`。フォルダ名を自動修正しない。Mod IDは `tconstruct` を基本とし、フォルダ名から自動生成されたIDを無検証で使わない。
- 本体には汎用ホストAPI、Mod側には原作固有の素材・計算・trait・recipe・機械ルールを置く。
- 詳細な着手順序と検証条件は `docs/tconstruct-implementation-plan.md` を参照する。提案段階のAPIを実装済みとして扱わない。
