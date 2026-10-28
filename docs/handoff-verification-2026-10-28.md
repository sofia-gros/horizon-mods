# 引き継ぎ後の限定検証（2026-10-28）

対象: Horizon main `a731fde7d75a9ee5aec7c9b702ac631b70393ceb` と horizon-mods `7884128fe9de2e350a2e22f4d2f3266faf24c897`。追加 sandbox 試験は Horizon `acaac87ecb59ff4e22d9a592669e3cd39685315b`。両 main は変更していない。

- Horizon `git diff --name-only 81b3be3791171700c8105e5b0b218dfa434a12a1 a731fde7d75a9ee5aec7c9b702ac631b70393ceb -- crates/horizon-items crates/horizon-world crates/horizon-client-core` は空。固定依存と最新本体の対象 crate は同一ソース。依存の固定は変更していない。
- Mod を一時コピーし、その dev-dependency `horizon-items` のみ `/workspace/horizon/crates/horizon-items` へ差し替え、`cargo test --manifest-path <一時コピー>/server-tools/Cargo.toml --test crafting_transaction` を実行: 4件成功。パス依存のため一時 lockfile は再解決された。ゲーム全体の互換性は対象外。
- Horizon `cargo test --locked -p horizon-mods --features runtime,pack --test sandbox`: 35成功、1 ignored。既存の重複 Mod ID 拒否、採掘・近接 callback の所有者分離と trap 後の他 Mod 継続に加え、異なる Welcome と mining fixture の同名 server command について両ロード順の先着優先と別名の継続を検証。初回は fixture の権限不足で1件失敗し、宣言を修正後に全体を再実行した。
- mining fixture は試験用で TConstruct 本体ではない。二つの mining owner は同じ fixture の別 ID であり、同一アイテムへの複数 Mod による同期変更、実ゲームでの登録競合や全 Mod 組合せは検証していない。
- `cargo fmt --all -- --check` と `git diff --check` は成功。
- horizon-mods には `.github/workflows` がなく、GitHub Actions API も workflow 0 件を返した。Horizon の checks は main push / PR / 手動実行で、普通の開発ブランチ push では実行されない。追加試験の Horizon commit で checks を手動起動した: https://github.com/sofia-gros/horizon/actions/runs/40616286929 。起動時点では queued、最終結果は別途確認が必要。
- 独立レビュー（Codex terra medium）は Mod 差分に重大な新規欠陥なしとした。手書き slot change による in-memory 試験であり UI、実ゲームの同時操作・保存は保証しない、と指摘した。
- 同じ独立レビュー担当は Horizon の追加差分も確認し、重大な欠陥なし、sandbox を独立に再実行して 35成功・1 ignored。実ゲーム command tree、reload 後の所有権、異なる引数の衝突は対象外。
