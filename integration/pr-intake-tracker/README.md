# 265件のPRと現在の統合状態

基準は [d01ffa の固定265件対応表](https://github.com/sofia-gros/horizon-mods/tree/d01ffa1931c49e30d6ee7ac8238cc4fbb671c441/pluto-lib/docs/pr-reorganization/full-coverage-20261106) です。同ディレクトリの4ファイルを変更せず保存し、今回の状態は `tracker.json` に分離しました。

現在照合した実装は `9cc1a01632913f01df1f0bcbb0b74b8bb9e80525` です。

- **明示的なPR単位のsource取り込み確認: 2件（#944、#945）**。その後の共通基盤・採掘・Guide・outer NBT変更は別のintake記録にあります。PR全機能の最新実機検証完了を意味しません。
- **個別PRとしての取り込み状態未照合: 263件**。基盤/main/commonに関連実装が含まれている可能性があるため、「機能がない」「未実装」とは扱いません。
- 265行すべてについて番号の一意性、固定head、reported base、own-delta base、diff digestを正本へ照合しました。最新265件の再検索ではなく固定snapshotへの追跡表です。
- この統合版の完全plan、12guard全通過、Game build、Windows link、ゲーム起動はまだ未実行です。過去の単体fixture成功をこの欄へ転記していません。
- 旧PRのclose、retarget、mergeは行っていません。ユーザーの最新指示により、必要なbuild/エラー確認を満たした後の統合先はmainです。未検証のままmainへ移しません。

## 更新方針

共通基盤、item/material/data、recipe/station/advancement、tool/combat、armor/defense/movement、machine/storage、world/fluid/rendering、Guide/UI/LANの大分類ごとに、実際のown deltaと現在の実装を照合します。元の7調査区分はこの8開発分類とは別なので、機械的に完了数へ置き換えません。

実際に採用したsourceの固定SHA、重複や依存の解消、限定fixture、全構成の検証をそれぞれ追記します。最優先は現在版のplan/guard/apply/buildと可能な起動エラー確認です。許可された一時欠落はpick-block時の色引き継ぎだけで、保存済み色は消していません。

[詳細な統合記録](../playtest-20261106.json) と [現状・未解決項目](../../PLAYTEST_STATUS.md) も参照してください。
