# 参照元と更新

最新の取り込み記録は [Horizon上流の追従記録](horizon-upstream-sync.md) を参照。以下は初回調査時点の記録で、server API 0.3は後続の `88b7bfaf` で一部実装済みになりました。

確認日: **2026-10-26（日本時間）**。
調査対象は上流 `main` のコミット **`9e447a7e9203eefdf5204eaa3079be0a2ea6f4df`** です。
ドキュメント作成時に上流のソースを取得し、ガイド・生成テンプレート・実装を照合しました。
Mod自体はまだ作成しておらず、この作業場所でCLIのインストール、Modのビルド、ゲーム起動は行っていません。

## 固定した一次資料

以下は調査対象コミットへのリンクです。

| 資料 | 用途 |
| --- | --- |
| [README](https://github.com/ludovic111/horizon/blob/9e447a7e9203eefdf5204eaa3079be0a2ea6f4df/README.md) | Horizon概要、本体の準備 |
| [Modガイド](https://github.com/ludovic111/horizon/blob/9e447a7e9203eefdf5204eaa3079be0a2ea6f4df/docs/mods.md) | 設計、作成、テスト、配布 |
| [Mod API概要](https://github.com/ludovic111/horizon/blob/9e447a7e9203eefdf5204eaa3079be0a2ea6f4df/docs/api/mods.md) | APIの全体像、将来計画 |
| [APIリファレンス](https://github.com/ludovic111/horizon/blob/9e447a7e9203eefdf5204eaa3079be0a2ea6f4df/docs/api/mods-reference.md) | 型と関数の一覧 |
| [WIT契約](https://github.com/ludovic111/horizon/blob/9e447a7e9203eefdf5204eaa3079be0a2ea6f4df/crates/horizon-mod-sdk/wit/horizon.wit) | 正確なimport/exportと型 |
| [Rust SDK](https://github.com/ludovic111/horizon/blob/9e447a7e9203eefdf5204eaa3079be0a2ea6f4df/crates/horizon-mod-sdk/src/lib.rs) | Mod trait、Registration、export macro |
| [CLI実装](https://github.com/ludovic111/horizon/blob/9e447a7e9203eefdf5204eaa3079be0a2ea6f4df/crates/cargo-horizon-mod/src/main.rs) | new/build/run/test/checkの実際の挙動 |
| [生成テンプレート](https://github.com/ludovic111/horizon/tree/9e447a7e9203eefdf5204eaa3079be0a2ea6f4df/crates/cargo-horizon-mod/template) | コード・コンテンツModの開始点 |
| [manifest実装](https://github.com/ludovic111/horizon/blob/9e447a7e9203eefdf5204eaa3079be0a2ea6f4df/crates/horizon-mods/src/manifest.rs) | API、権限、ID、名前空間の検証 |
| [コンテンツ検証](https://github.com/ludovic111/horizon/blob/9e447a7e9203eefdf5204eaa3079be0a2ea6f4df/crates/horizon-mods/src/content.rs) | パッケージ内の許可パス・制限 |
| [テストランナー](https://github.com/ludovic111/horizon/blob/9e447a7e9203eefdf5204eaa3079be0a2ea6f4df/crates/horizon-mods/src/testing.rs) | TOMLシナリオとexpectの仕様 |
| [公式Modの実装](https://github.com/ludovic111/horizon/tree/9e447a7e9203eefdf5204eaa3079be0a2ea6f4df/mods/official) | FPS・キー表示・zoom・minimapの利用例 |

## 説明と実装の差異

上流文書には過去の説明や計画が残っています。このガイドでは次のように扱いました。

- API概要の機能数や「未実装」一覧だけで判定しません。`ui`・`render`・`input` はWITとmanifestに存在し、実験APIとして利用できます。
- `registries` は計画として書かれた節もありますが、API `0.2` のコンテンツModはテンプレートとビルド処理に実装されています。
- サーバーModのコードフックや将来のAPI `0.3`、任意のワールド書き換え・インベントリAPIなどは、現時点で利用可能としていません。
- `.hmod` のassets/dataを「後で追加」とする古い記述より、現在のコンテンツパッケージ実装を優先しています。
- 本体READMEやCLI package単独の最小Rust表記より、実際に依存する `horizon-mods` のRust 1.95要件を優先しています。

## 更新方法

対象Horizonを更新するときは、まず新しいリリースのコミットを特定します。
WIT、SDK、manifest、CLI、テンプレートを比較し、実装済みの機能と実験APIの変更を確認します。
その後、対象ModのSDK `rev` とlockfileを更新し、ビルド・シナリオテスト・成果物check・ゲーム内確認を行います。
成功した対象をModのREADMEに記録し、共通ガイドの確認日と参照コミットも更新します。

現在の案内を確認する入口: [公式リポジトリ](https://github.com/ludovic111/horizon)、[公式ドキュメント](https://playhorizon.dev/docs)、[Mod作成ガイド](https://playhorizon.dev/docs/make-a-mod)。

## 追加調査

[Create・IC2・BuildCraftの移植調査](porting-create-ic2-buildcraft.md) は、同日のHorizon `45bc45b3967f06ab56173fe2b13892004e717e35` を対象にしています。
WIT・SDK・manifest・content定義・Modpack計画に、上記の初回調査コミットとの差分がないことを確認しました。
各Mod側の調査対象と根拠は、その報告に記載しています。

[Tinkers’ Constructの移植調査](porting-tinkers-construct.md) は、TConstruct `1.20.1` の `f4e77d43f39ba1047bf93ae816b7e1beaab514c2` と、同日のHorizon `45bc45b3967f06ab56173fe2b13892004e717e35` を対象にしています。
Mantleの補助調査対象、依存の指定バージョン、未検証事項も報告内に記録しています。
