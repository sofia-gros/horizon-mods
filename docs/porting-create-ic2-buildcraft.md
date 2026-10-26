# Create・IC2・BuildCraftのHorizon移植可能性

調査日: **2026-10-26（日本時間）**。公開ソース・公式文書による静的調査です。移植コードの作成、ビルド、ゲーム内検証は実施していません。

## 結論

**現行Horizonの公開Mod APIだけでは、3つとも機械として完全には移植できません。**
Horizon本体を拡張し、その上でRust / Wasm向けに機能を再実装するなら技術的な道筋はあります。
ただし、その拡張と再実装の成立を実機で検証したわけではありません。

| 対象 | 現行APIのまま | 本体・APIの拡張後 | 主な障壁 |
| --- | --- | --- | --- |
| Create | 装飾ブロック・通常アイテム・対応するvanilla形式のレシピなどの一部 | 固定機械から段階的に再実装可能。全体は大規模 | 回転動力網、機械状態、ワールド描画、動く構造物、衝突、同期 |
| IC2 | 素材・装飾・対応する通常レシピなどの一部 | 静的な機械群は初期の検証対象に向く | サーバーtick、スロット、EU網、機械レシピ、保存と同期、原作の利用条件 |
| BuildCraft | 素材・装飾などの一部 | 輸送処理から段階的に再実装可能 | アイテム・液体・MJ輸送、搬送物の表示、Quarryのワールド操作・描画 |

「素材が登録できた」「モデルが表示できた」は機械が動くことの証明にはなりません。
既存Javaのjarをそのまま実行すること、Forge / NeoForgeのクラスをWasmへ自動変換して動かすことは、現行Horizonの仕組みではできません。[H1][H2][H3]

## 対象バージョンと調査範囲

ユーザー指定の「craete」は **Create**、「EntityBlock」は主に **Block Entityとそのアニメーション描画** を指すものとして調査しました。
Modのバージョン指定がないため、次の対象を用いました。

| プロジェクト | 調査対象 |
| --- | --- |
| Horizon | `main` の `45bc45b3967f06ab56173fe2b13892004e717e35` |
| Create | `mc1.21.1/dev` の `7ff2760e4924d35d4bc2e22c4b19923f0178cd16`。gradle.propertiesは6.0.11、Minecraft 1.21.1、NeoForge |
| BuildCraft | デフォルトブランチ `8.0.x-1.12.2` の `7c86626d09c4569fb2e0f458a16aaf8c3148ebaf`。build.propertiesは8.0.1-pre.2、Minecraft 1.12.2。開発系列であり7.99系の全挙動を検証したものではない |
| IndustrialCraft² | 公式配布ページのForge / Minecraft 1.12.2系列と公式Wiki。IC2 Classicなど別実装は対象外。内部ソース全体は取得・検証していない |

先に作成した共通ガイドはHorizon `9e447a7…` を対象としています。
今回のHorizonコミットと、そのコミットのWIT・SDK・manifest・content定義・`docs/modpacks.md` を比較し、該当ファイルに差分がないことを確認しました。
この報告の判定は新しい上記コミットを基準にしています。

## Block Entityとアニメーションは別の要件

Block Entity（旧版のTileEntity）は、特定の座標にあるブロックの追加状態を管理する仕組みです。
機械のインベントリ、処理進捗、蓄電量、液体タンクなどを持ちます。必要に応じてtickで処理し、保存・読み込み・クライアント同期を行います。
見た目が普通の立方体でも、この仕組みは必要になります。

一方、`ENTITYBLOCK_ANIMATED` やBlock Entity Renderer / 旧TESRは描画側の仕組みです。
**アニメーションを使わないことは、Block Entityを使わないことを意味しません。**

- Createの `FlywheelBlock` は `RenderShape.ENTITYBLOCK_ANIMATED` を返します。`FlywheelVisual` はFlywheelライブラリのinstanceを使い、部分モデルの回転を更新しています。[C1][C2]
- IC2は公式開発者の説明でもTileEntityのクライアント・サーバー更新処理を使っています。電気炉の公式仕様にもスロット・EU・内部蓄電があり、静的な外観でも状態と処理が必要です。[I1][I2]
- BuildCraftの `TilePipeHolder` と `TileQuarry` はtick可能なTileEntity系のクラスです。Quarryには `TileEntitySpecialRenderer`、エンジンには `FastTESR` を使う描画処理があります。[B1][B2][B3][B4]

したがって「CreateだけがBlock Entityを使う」「BuildCraftは専用アニメーションを使わない」という分類では、移植難易度を正しく見積もれません。
ただし、Horizon向けの新実装でJavaと同じクラス構造を再現する必要はありません。座標ごとの状態をチャンク単位でまとめるなどの設計は可能です。その場合も状態の寿命・保存・同期の役割は必要です。

## Horizon側の実装確認

| 必要な仕組み | 調査時点の公開Mod API | 移植への影響 |
| --- | --- | --- |
| ブロック・アイテム登録 | API 0.2でvanillaテンプレートから登録 | 素材や装飾の一部は作れる。独自の機械動作は定義できない |
| クライアントのtick / frame | あり | サーバーが管理する機械のtickとは異なる |
| 独自ブロックのuse / place / break / neighbour等 | 公開WITに未提供 | 設置・操作・接続変更に対する機械処理が組めない |
| 独自Block Entity型、座標ごとの永続状態・tick | 公開WITに未提供 | 機械・パイプ・動力源の基盤が足りない |
| ワールド、ブロック、スロットへの汎用アクセス | 公開WITに未提供 | 加工入出力、採掘、隣接機械との接続が足りない |
| エネルギー・アイテム・液体handler | 公開WITに未提供 | EU / MJ / パイプ輸送をゲームへ接続できない |
| UI | 実験APIの画面・widgetあり | サーバー側スロット・転送・トランザクションを備えた機械menuの代用にはならない |
| render | 実験APIのHUD、zoom、minimap等 | 任意の3Dモデルをワールド内に配置・回転させるAPIではない |
| 独自Entity型、動くブロック集合、衝突 | 公開WITに未提供 | Createのcontraptionや列車に不足 |
| storage | Mod専用key/valueあり | ワールド・dimension・座標・チャンク寿命と統合した機械ストレージではない |
| command | プレイヤー権限でvanillaコマンドを予約実行 | 読み書きと輸送の汎用APIではなく、通常のマルチプレイ機械処理の代替にはならない |

根拠はWITの全interface、manifestが受理するAPI、contentの定義とロード処理です。[H1][H2][H3][H4]
本体にvanillaのBlock Entity処理・アニメーション描画は存在しますが、任意のModへ登録・呼び出し口が公開されているわけではありません。[H6]

上流計画では、behaviour / serverを0.3、Block Entity・機械menu・handler・ネットワークを0.4、Entity・カスタム描画を0.5に分類しています。
**これは計画上の区分であり、提供済みAPIやリリース日の約束ではありません。** 調査対象のmanifestはAPI `0.1` と `0.2` だけを受理します。[H2][H5]

## Mod別の移植方針

### Create

固定機械でも、回転数・回転方向・歯車比・動力源・stress / capacity・ネットワーク更新が必要です。
`KineticBlockEntity` と `KineticNetwork` は、描画と別に機械状態と負荷を管理しています。[C3][C4]
歯車のアニメーションだけを追加しても加工装置としては動きません。

可動構造物はさらに別段階です。`Contraption` は複数ブロックや搭載storageをまとめ、Entityとして移動させる仕組みと組み合わされます。
`ContraptionCollider` は移動・回転する構造物とEntityとの衝突を扱います。[C5][C6]
列車や移動装置を含む忠実な移植には、組み立て・分解、座標変換、衝突、搭載インベントリ、保存、同期が必要です。
単なる骨格アニメーションや回転行列APIだけでは満たせません。

現実的な順序は、固定された軸と歯車 → 回転動力網 → 固定加工装置 → ベルトや搬送 → 可動構造物 → 列車などです。
元のFlywheel描画コードをHorizonの描画系へそのまま接続することはできません。
Flywheel自体の完全移植は必須ではなく、Horizon側に部分モデルのtransform・instancing・照明・cullingを持つ同等機能を用意する方法が考えられます。これは設計上の提案です。

### IndustrialCraft²（IC2）

初期検証には、発電機・ケーブル・蓄電器・電気炉のような静的機械の小さい組み合わせが向いています。
描画面ではCreateの可動構造物より範囲を絞れますが、サーバー側の機械基盤が必須なのは同じです。

最低限、入力・出力スロット、面ごとの搬入搬出、EU受給、内部蓄電、加工進捗、保存と同期が必要です。
忠実に再現するなら、EUの電圧階層、過電圧、ケーブル損失等の対象バージョンのルールも調査・実装します。
単一のFE風残高に置き換えただけではIC2互換になりません。公式電気炉の説明にも入力電圧と過電圧の扱いがあります。[I1]

IC2内部の完全なコードレビューはしていないため、全機械・原子炉・農業・装備・液体・旧セーブ変換までの網羅性や工数は確定できません。
また、IC2 ClassicとExperimentalでは仕様・実装・利用条件を分けて再調査する必要があります。

### BuildCraft

最小構成は、チェスト → 抽出 → アイテムパイプ → チェストです。
この段階でも接続面、経路選択、移動中のItemStack、行き先満杯時の処理、ドロップ、チャンクをまたぐ保存が必要です。
公開ソースではアイテム・液体・電力に別の `PipeFlow` 実装があり、輸送規則を区別しています。[B5]

次に液体・MJ・エンジン・gate類、最後にQuarryやbuilder類へ進めるのが妥当です。
Quarryは機械tickだけでなく、採掘・loot・インベントリ排出・稼働領域・チャンクロードと、枠やドリル等の描画を必要とします。[B2][B3]
対象ソースにはMJとForge Energy関連の処理があり、バージョンごとの変換・接続規則を確認する必要があります。[B5]

したがって、BuildCraft全体を「アニメーションを使わない簡単なMod」と扱うことはできません。
ただし、最初のアイテム輸送だけなら、Createの動く構造物より狭い範囲で検証できます。搬送中の可視化を省いた試作は、原作の外観まで移植したものとは分けて記録します。

## 実装を始める場合の選択肢

| 選択肢 | 実行できること | 評価 |
| --- | --- | --- |
| 現行 `.hmod` のみ | 素材・装飾・対応する通常レシピ、ローカルUI等 | 今すぐ着手できるが、工業Modの機械移植には届かない |
| Horizon本体をforkし、Mod APIを追加 | サーバー処理・機械状態・輸送・描画を順に公開し、ModをWasm側へ実装 | 複数Modを育てる目的に合う。API設計・本体実装が先行作業 |
| 本体に機械処理を直接実装 | 既存のRust内部処理へ接続した専用ビルド | 実験は可能だが通常の `.hmod` だけで配布できず、本体への追従が必要 |

この作業場所で複数Modを開発するなら、**共通の機械基盤をHorizon側に追加し、各Modのルールを分離する方式**を提案します。
単一 `.hmod` にライブラリを同梱するだけでは、ホストにないワールド・機械・描画APIは増やせません。
Mod間の共有handlerも、ホスト側の型・所有権・呼び出し規約が必要です。

### 共通基盤で先に決めること

1. サーバーが管理するブロックイベントと機械tick。チャンクload / unloadと、無人時・専用サーバーでも動く条件。
2. dimensionと座標に結びつく状態、schemaのバージョン、保存・復元、破壊時の中身の排出。
3. item / fluidの挿入・取り出し。事前照会と確定処理、面制限、途中失敗時の巻き戻し、同時操作による複製防止。
4. EU・MJ・回転動力を区別できるネットワークと資源型。各Modのルールを単一のFE仕様へ潰さない。
5. スロットと操作をサーバーで検証するmenu、進捗表示、クライアント同期、Mod間の接続。
6. ワールド内部分モデルとtransform、補間、照明、描画範囲、Instancing。可動構造物は別途衝突と同期も設計する。
7. sandboxの権限・fuel・メモリ上限。大量の機械に対して1台ずつ高頻度に境界を跨ぐ方式を避け、batch化と計測を行う。

これらは調査から導いた追加要件で、既存Horizon APIの機能名や完成済み設計ではありません。
未計測のため「Rustなら必ず速い」「大規模工場も64 MiBで収まる」とは判断できません。

### 最初に成立を確認する試作

| 段階 | 試作 | 合格条件 |
| --- | --- | --- |
| 1 | 独自アセットの静的加工機1台 | サーバーtickで入力を消費し出力を生成。保存・再起動で進捗と中身が残る。通常操作で複製・消失しない |
| 2 | 発電機・ケーブル・蓄電器・加工機 | 接続・切断・chunk unload / reload後も収支が一致する。対象方式の電圧や損失等を検証できる |
| 3 | チェスト間のアイテムパイプ | 分岐・満杯・破壊・途中保存でもアイテム総数が一致する |
| 4 | 軸と歯車の回転 | サーバーの速度・向き・負荷と描画が一致。多数設置時のtickと描画時間を計測する |
| 5 | 複数ブロックの回転・移動 | 衝突、乗ったEntity、積載物、組み立て・分解・保存・同期が成立する |

1〜3が通ると静的な工業Modへ進める根拠になり、4〜5が通るとCreateの主要な追加難所を検証できます。
これは技術検証の順序であり、IC2やBuildCraftの完全互換をこの5段階だけで保証するものではありません。
原作との一致は、選んだ正確なバージョンで同じ入力・tick数・保存操作を行い、出力・状態を比較して判断します。

## ソース・アセットを再利用する際の条件

技術的な移植可否と、原作ファイルを含む公開版を配布できるかは分けて扱います。

- **Create:** 調査コミットの `LICENSE.md` はコード等がMIT、`src/main/resources/assets/` 配下がAll Rights Reservedです。コードの条件だけでテクスチャ・モデル・音なども再配布可能とは扱えません。原作アセットを使う場合は対象条件の確認・必要な許諾、または独自アセット化が必要です。[C7]
- **BuildCraft:** 調査した主要クラスはMPL-2.0のヘッダーを持ち、`LICENSE-NEW` もMPL-2.0です。一方で旧ライセンスの `LICENSE` と旧コードも併存します。利用するファイル・resource・submodule単位で条件を追い、MPL対象の派生部分は該当するソース提供・通知条件に従う必要があります。全ファイルを一括してMIT扱いにはできません。[B6]
- **IC2:** 公式配布ページはARR、公式Wikiには改変・公開への制限と例外の説明があります。Modpackとして原版を配る条件は、Rust移植版のコードやアセットを配布する許諾とは別です。公開移植の許諾が得られたとは判断していません。実移植前に対象版の条件を確認し、原作コード・アセットを使う方針なら権利者への確認が必要です。[I3][I4]

本調査では権利者への連絡・公開申請は行っていません。独自仕様・独自コード・独自アセットによる工業Modの試作と、原作そのものの移植は区別して進めます。

## 一次資料

コードへのリンクは調査コミットに固定しています。IC2のWeb文書は更新される可能性があります。

- [H1: Horizon WIT](https://github.com/ludovic111/horizon/blob/45bc45b3967f06ab56173fe2b13892004e717e35/crates/horizon-mod-sdk/wit/horizon.wit)
- [H2: Horizon manifest・API検証](https://github.com/ludovic111/horizon/blob/45bc45b3967f06ab56173fe2b13892004e717e35/crates/horizon-mods/src/manifest.rs)
- [H3: Horizon content定義](https://github.com/ludovic111/horizon/blob/45bc45b3967f06ab56173fe2b13892004e717e35/crates/horizon-mods/src/content.rs)
- [H4: Horizon本体のcontent取り込み](https://github.com/ludovic111/horizon/blob/45bc45b3967f06ab56173fe2b13892004e717e35/crates/horizon-client/src/mod_content.rs)
- [H5: Horizonの移植要件・API計画](https://github.com/ludovic111/horizon/blob/45bc45b3967f06ab56173fe2b13892004e717e35/docs/modpacks.md)
- [H6: Horizon本体のBlock Entity描画](https://github.com/ludovic111/horizon/blob/45bc45b3967f06ab56173fe2b13892004e717e35/crates/horizon-client/src/block_entity_render.rs)
- [C1: Create FlywheelBlock](https://github.com/Creators-of-Create/Create/blob/7ff2760e4924d35d4bc2e22c4b19923f0178cd16/src/main/java/com/simibubi/create/content/kinetics/flywheel/FlywheelBlock.java)
- [C2: Create FlywheelVisual](https://github.com/Creators-of-Create/Create/blob/7ff2760e4924d35d4bc2e22c4b19923f0178cd16/src/main/java/com/simibubi/create/content/kinetics/flywheel/FlywheelVisual.java)
- [C3: Create KineticBlockEntity](https://github.com/Creators-of-Create/Create/blob/7ff2760e4924d35d4bc2e22c4b19923f0178cd16/src/main/java/com/simibubi/create/content/kinetics/base/KineticBlockEntity.java)
- [C4: Create KineticNetwork](https://github.com/Creators-of-Create/Create/blob/7ff2760e4924d35d4bc2e22c4b19923f0178cd16/src/main/java/com/simibubi/create/content/kinetics/KineticNetwork.java)
- [C5: Create Contraption](https://github.com/Creators-of-Create/Create/blob/7ff2760e4924d35d4bc2e22c4b19923f0178cd16/src/main/java/com/simibubi/create/content/contraptions/Contraption.java)
- [C6: Create ContraptionCollider](https://github.com/Creators-of-Create/Create/blob/7ff2760e4924d35d4bc2e22c4b19923f0178cd16/src/main/java/com/simibubi/create/content/contraptions/ContraptionCollider.java)
- [C7: Createのコード・アセット別ライセンス](https://github.com/Creators-of-Create/Create/blob/7ff2760e4924d35d4bc2e22c4b19923f0178cd16/LICENSE.md)
- [B1: BuildCraft TilePipeHolder](https://github.com/BuildCraft/BuildCraft/blob/7c86626d09c4569fb2e0f458a16aaf8c3148ebaf/common/buildcraft/transport/tile/TilePipeHolder.java)
- [B2: BuildCraft TileQuarry](https://github.com/BuildCraft/BuildCraft/blob/7c86626d09c4569fb2e0f458a16aaf8c3148ebaf/common/buildcraft/builders/tile/TileQuarry.java)
- [B3: BuildCraft RenderQuarry](https://github.com/BuildCraft/BuildCraft/blob/7c86626d09c4569fb2e0f458a16aaf8c3148ebaf/common/buildcraft/builders/client/render/RenderQuarry.java)
- [B4: BuildCraft RenderEngine_BC8](https://github.com/BuildCraft/BuildCraft/blob/7c86626d09c4569fb2e0f458a16aaf8c3148ebaf/common/buildcraft/lib/client/render/tile/RenderEngine_BC8.java)
- [B5: BuildCraftのアイテム・液体・電力輸送](https://github.com/BuildCraft/BuildCraft/tree/7c86626d09c4569fb2e0f458a16aaf8c3148ebaf/common/buildcraft/transport/pipe/flow)
- [B6: BuildCraft MPL-2.0](https://github.com/BuildCraft/BuildCraft/blob/7c86626d09c4569fb2e0f458a16aaf8c3148ebaf/LICENSE-NEW) / [旧ライセンス](https://github.com/BuildCraft/BuildCraft/blob/7c86626d09c4569fb2e0f458a16aaf8c3148ebaf/LICENSE)
- [I1: IC2公式Wikiの電気炉仕様](https://wiki.industrial-craft.net/index.php?title=Electric_Furnace)
- [I2: IC2開発者のTileEntity更新処理の説明](https://forum.industrial-craft.net/thread/11755-remove-ic2-core-block-tileentityblock-updateentity-final-modifier/)
- [I3: IC2の公式配布ページ・ライセンス表示](https://modrinth.com/mod/industrial-craft)
- [I4: IC2公式Wikiの利用条件](https://wiki.industrial-craft.net/index.php?title=Industrial-Craft-Wiki:About)
