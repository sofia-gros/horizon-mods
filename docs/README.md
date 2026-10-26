# Horizon Mod 開発ドキュメント

このフォルダに複数Mod向けの共通知識をまとめます。各Modは `../{mod_name}/` に作成します。

| ドキュメント | 内容 |
| --- | --- |
| [構成と運用](repository-layout.md) | ディレクトリ、命名、依存固定、Modごとの記録 |
| [開発環境と作成手順](getting-started.md) | Windows / PowerShellを中心としたセットアップ、作成、実行 |
| [API・manifest](mod-api.md) | コードMod、権限、フック、実験API、制約 |
| [コンテンツMod](content-mods.md) | ブロック・アイテム・アセット・名前空間 |
| [検証・配布・トラブル対応](testing-and-release.md) | シナリオテスト、ゲーム内検証、配布 |
| [参照元と更新](upstream.md) | 調査対象コミット、一次資料、更新時の確認箇所 |
| [Create・IC2・BuildCraft移植調査](porting-create-ic2-buildcraft.md) | 機械・Block Entity・アニメーションの対応可否、不足API、実装方針 |
| [Tinkers’ Construct移植調査](porting-tinkers-construct.md) | MIT・Mantle依存、素材とツール、溶融炉、不足API、段階的な移植案 |
| [TConstructの詳細実装調査](tconstruct-implementation-plan.md) | ローカルフォークの関数・保存・サーバー経路、API契約案、作業順序、検証条件 |
| [検証自動化の調査・実機確認](verification-automation.md) | Prism・launcherの配置、Windows CLI実行、保存復帰の実証、再実行スクリプト |
| [移植ロードマップ](tconstruct-roadmap.md) | 実装段階・完了条件・手動試験の扱い |
| [TConstruct M0・M1検証記録](tconstruct-m1-verification.md) | コア・Wasm・sandbox・ゲーム内診断の結果と制限 |
| [TConstruct M2検証](tconstruct-m2-verification.md) | 専用ブランチ、上流PR統合、server lifecycle probeと起動手順 |
| [TConstruct M3a検証](tconstruct-m3a-verification.md) | 個体状態の保存・通信、vanilla試験アイテム、再開できる手動テスト |
| [TConstruct M3b検証](tconstruct-m3b-verification.md) | サーバーの手持ち読み取り、SDK固定、個体ごとの状態・基礎statsの診断 |
| [Horizon上流の追従記録](horizon-upstream-sync.md) | API 0.3の追加範囲、ゲーム側の未接続部分、継続確認手順 |

## Horizonの前提

HorizonはMinecraft: Java EditionをRustで再実装したプロジェクトです。
独自ModはWebAssemblyコンポーネント、またはデータ主体の `.hmod` として読み込まれます。
Fabric / NeoForge向けJavaの `.jar` をそのまま実行する仕組みではありません。

調査時点の実装では、コードModはAPI `0.1`、コンテンツはAPI `0.2` に対応します。
後続の `88b7bfaf` では実験的なserver API `0.3` の契約・SDK・runtimeが追加されました。ゲームからの呼び出しは未接続です（[追従記録](horizon-upstream-sync.md)）。
サーバー用コードフックなどの将来計画と、現状で使える機能は分けて扱います。

確認日: **2026-10-26（日本時間）**。対象コミット: `9e447a7e9203eefdf5204eaa3079be0a2ea6f4df`。
このガイドはその時点の実装を基準とし、後のリリースへの対応を保証するものではありません。
詳細な出典は [参照元と更新](upstream.md) を参照してください。
