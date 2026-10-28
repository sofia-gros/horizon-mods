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

## 実 Mod と CI の追加確認

Horizon `acaac87ecb59ff4e22d9a592669e3cd39685315b`、Mod code `decd06e38b7663717e015bf17fc2b4e29ea3f93c` の組合せで、server-tools package を再ビルド（SHA-256 `778194bf845c2f2d2bd42bb21cf07ee3cc43d4ce480600e9945d8d0d57e06141`）。Horizon 標準 Welcome package の SHA-256 は `4fbcbb02a608216cc330382c3132aec00cc8f8af0e98c85a4772d80c6dbcd3d2`。

`tools/handoff_external.rs` を一時的に Horizon の `crates/horizon-mods/tests/handoff_external.rs` にコピーし、`TCONSTRUCT_HMOD=<server-tools packageの絶対パス> cargo test --locked -p horizon-mods --features runtime,pack --test handoff_external` を実行し1件成功。終了後コピーを削除。先に通常 sandbox を実行し Welcome と mining fixture の Wasm をビルドする必要がある。この補助試験は標準テストには自動登録されない。

実 TConstruct、Welcome、trap 用 mining fixture を同時ロードし、TConstruct の採掘・近接 hook が個別の damage 0 snapshot に対して各1・2の更新案を返すこと、Welcome の応答、fixture trap 後に両 Mod が動き続けることを確認。Welcome を TConstruct と同じ ID で再包装した場合、両ロード順で後着が拒否されることも確認。異なる実 Mod 間の同名 command 衝突ではなく、game 側のイベント配送、変更確定、保存・同期、同一アイテムへの複数 Mod の変更、連続耐久、通常インストール、3者の全ロード順は未検証。独立レビューがこれらの限界を確認した。

CI run 40616286929 の verify は `test_release_train.ReleaseTrainTests.test_a_game_release_opens_with_the_way_to_the_launcher` が fork の URL と固定された upstream URL の不一致で失敗。今回変更箇所と無関係。ローカル `python3 -m unittest discover -s tests -v` は398件成功（4 skipped）だが CI 成功の代替ではない。launcher は成功、手動起動で追加された `vanilla-parity` と `jfr-analysis` は専用 self-hosted runner 待ち。runner 一覧 API は権限不足の HTTP 403、完了時刻は不明。CI を重複起動していない。
