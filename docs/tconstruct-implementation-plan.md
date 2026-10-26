# Tinkers’ Construct: ローカルフォークを使う詳細実装調査

後続の [検証自動化調査](verification-automation.md) で、端末にあるPrismの26.3・Java 25・準備済みresourceを確認し、リリース版Horizonで保存復帰を含む自動smokeを実行しました。下記の環境記述は、この詳細調査時点のcheckout内の状態です。

調査日: **2026-10-26（日本時間）**。これは実装計画であり、以下に示す新API・型・ファイルは、既存と明記したもの以外は提案です。
追記: 同日の上流 `88b7bfaf` でserver用WIT・SDK・runtimeが実装されました。下記の初回調査にあるserver拒否の記述は旧版についての記録です。現行の実装判断と残作業は [上流追従記録](horizon-upstream-sync.md) と [ロードマップ](tconstruct-roadmap.md) を優先します。
コード変更・依存インストール・ゲーム起動・ビルド・ベンチマークは実施していません。

## 1. 作業対象と決定事項

| 対象 | 場所・基準 |
| --- | --- |
| Horizon本体 | `A:\Project\horizon`。originは `https://github.com/sofia-gros/horizon.git` |
| 本体の確認コミット | `45bc45b3967f06ab56173fe2b13892004e717e35`。調査開始時の作業ツリーに変更なし |
| Mod開発場 | `A:\Project\horizon-mods` |
| 今回のMod | ユーザー指定どおり `A:\Project\horizon-mods\Tinkers' onstruct` |
| 原作ソース | TConstruct `1.20.1`、`f4e77d43f39ba1047bf93ae816b7e1beaab514c2` |
| 依存・ライセンス等の先行調査 | [Tinkers’ Construct移植調査](porting-tinkers-construct.md) |

フォルダ名は空白とアポストロフィを含む指定名をそのまま使います。**フォルダ名とMod IDを分離し、ID / namespaceは `tconstruct`、Cargo packageは `tconstruct-horizon` を明示する**案です。
CLIの `new` はフォルダ名からIDを自動生成するので、指定フォルダ名の自動変換結果をそのまま使いません。

本体には複数Modで使えるホスト機能を追加し、原作固有の素材・計算・trait・レシピ・機械ルール・アセットはMod側へ置きます。
現行のWasm sandboxを維持し、Java・Forge・Mantleのバイナリをゲームへ埋め込む方式は採用しません。
上流へ追従するため、本体にTConstruct専用の分岐やJavaコードの翻訳を増やさない構成にします。

本体の [CONTRIBUTING.md](A:/Project/horizon/CONTRIBUTING.md) はMod等のソースから本体コードを派生させない方針を持っています。
今回の依頼はMod移植を明示的に許可していますが、同期・将来の上流提案を考慮し、原作から移植したMITコードはMod側に置き、本体の汎用APIはHorizonの既存構造と独立した契約から実装する方針です。

## 2. 調査で具体化・修正された点

### 保存機構はゼロから作る必要がない

既存の `ItemStack` は `item / count / damage / extra` を持ち、`StackData.other` は未モデル化componentをNBTとして保持します。
`other()` / `set_other()`、`encode_components()` / `decode_components()` が既にあります。
したがって、**本体のRustデータ構造には任意の追加データを保持する基盤が存在します**。不足するのはModへの限定的な公開と、全経路での保存・同期・運用です。

一方、送信側の `lan::guest::network_stack()` はcomponentを個別に組み立てており、任意の `other` 全体を自動送信する処理にはなっていません。
プロトコル層は `CUSTOM_DATA` とNBTを扱い、受信側の `item_sync` はNBTから保存用componentへ戻せます。
**ローカル保存が可能でも、ネットワーク往復で保持されると仮定してはいけません。** 最初に専用テストで確定します。

参照: [ItemStack](A:/Project/horizon/crates/horizon-client/src/items.rs:212)、[追加component](A:/Project/horizon/crates/horizon-client/src/item_data.rs:12)、[送信](A:/Project/horizon/crates/horizon-client/src/lan/guest.rs:488)、[受信](A:/Project/horizon/crates/horizon-client/src/item_sync.rs:14)、[protocol](A:/Project/horizon/crates/horizon-protocol/src/item.rs:14)。

### サーバーでModを実行する経路が最優先

- `horizon-mods::runtime::Mods::start` は `side=server` を `server mods do not run yet` で拒否します。
- `game/mods.rs::start_mods` は現在のローカルUI・設定と結びついたロード処理です。
- `open_integrated` はサーバー用optionsを作る際に `options.dev_mods.clear()` します。
- 既定のシングルプレイはサーバースレッド方式。ウィンドウ側の `tick()` は `remote` があれば `tick_remote()` へ戻り、ローカルのワールド処理は走りません。
- サーバーはdimensionごとに `Parked` を入れ替えて処理します。既存 `tick_mods()` に機械を足すだけでは、全dimensionの正しいtick・pause・保存を保証できません。

参照: [runtime](A:/Project/horizon/crates/horizon-mods/src/runtime.rs:292)、[Modロード](A:/Project/horizon/crates/horizon-client/src/game/mods.rs:260)、[内蔵サーバーへの引き渡し](A:/Project/horizon/crates/horizon-client/src/game/integrated.rs:190)、[tick](A:/Project/horizon/crates/horizon-client/src/game.rs:4200)、[dimensionのtick](A:/Project/horizon/crates/horizon-client/src/game/levels.rs:295)。

### 木材つるはしもtrait依存を持つ

選んだ原作コミットのpickaxeには `pierce` があり、数値補正だけでなく `weapon_mob_effect` に依存します。
woodの `cultivated` は修理係数のmoduleです。flintの `jagged` は失った耐久等に基づく条件付き採掘・攻撃補正を含みます。
この版の素材definition / stats / traitsに単純な `stone.json` はありません。vanillaの採掘tier `minecraft:stone` と素材IDは区別します。

初期の調査対象は **woodとflint、pick_head / tool_handle / tool_binding、pickaxe** を推奨します。
ただし、最小の原作忠実実装にもtraitと攻撃の接続が必要です。採掘だけのデモを先に作る場合は、攻撃未対応の技術試作として明記します。

参照: [pickaxe定義](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/generated/resources/data/tconstruct/tinkering/tool_definitions/pickaxe.json)、[cultivated](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/generated/resources/data/tconstruct/tinkering/modifiers/cultivated.json)、[jagged](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/generated/resources/data/tconstruct/tinkering/modifiers/jagged.json)、[pierce](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/generated/resources/data/tconstruct/tinkering/modifiers/pierce.json)。

## 3. 推奨する構成

```text
A:\Project\horizon\
  crates/horizon-mod-sdk/     WIT・Rust SDK・export macro
  crates/horizon-mods/        manifest・sandbox・host型・runtime・headless試験
  crates/horizon-client/      ゲームへ接続する汎用アダプター
  crates/horizon-assets/      汎用のモデル・描画情報の解釈
  crates/horizon-protocol/    必要なcodecと既存custom payloadの利用

A:\Project\horizon-mods\Tinkers' onstruct\
  Cargo.toml                 単一Modのcdylib package（実装時に作成）
  Cargo.lock
  horizon-mod.toml
  src/
    lib.rs                   将来のWasm adapter
    core/                    素材・部品・stats・trait・tool状態
    adapter/                 host DTOへの変換、event応答
    stations/                部品作成・組み立て・修理
    smeltery/                後段の溶融・鋳造・合金
  data/tconstruct/            対応済みデータ・変換後の登録定義
  assets/tconstruct/          対応済みモデル・独自renderer用宣言等
  tests/
    core/                    Rustのゲーム非依存試験用module等
    fixtures/                Javaとの比較用入出力
  tools/                     オフラインのimport・監査・fixture生成
  third_party/               MIT本文・再利用ファイルの出典一覧
```

まずは単一Cargo package内でcoreとadapterをmoduleで分離すれば十分です。rootに共通workspaceは作りません。
コアが独立した依存として必要になった時点で、Modディレクトリ内の別crate化を検討します。
この調査ではModフォルダにREADMEのみを作成し、ビルド可能なModを装うmanifestや空のWasmを置いていません。

開発中のSDKは `path = "../../horizon/crates/horizon-mod-sdk"` が候補です。
これはModフォルダからの相対パスであり、ルートからのパスではありません。
配布・再現検証時は `https://github.com/sofia-gros/horizon` の**実装済みコミット**へ `rev` を固定します。未実装の現HEADへ新API依存を固定しません。
CLIは既存のグローバル版と混同せず、本体checkoutの `cargo-horizon-mod` をビルドして使います。

## 4. 本体側の変更箇所

| 作業 | 既存の入口 | 実装時の方針 |
| --- | --- | --- |
| 新API・権限 | `horizon-mod-sdk/wit/horizon.wit`、SDK `src/lib.rs`、`horizon-mods/src/manifest.rs` | サーバー用契約を分け、旧client guestを維持する |
| sandbox・役割 | `horizon-mods/src/runtime.rs` | runtime roleでclient / serverを選別。サーバー側はHUD・カメラ・ローカル入力を公開しない |
| ロード・開発実行 | `game/mods.rs`、`game/integrated.rs`、`game/dedicated.rs`、CLI `main.rs` | hostごとに必要なcomponentを1回ロード。開発packageをサーバーにも引き渡す |
| 個体データ | `items.rs::ItemStack`、`item_data.rs::StackData`、`item_sync.rs`、`lan/guest.rs` | 既存custom dataを利用。保存と送受信の往復を先に保証 |
| 採掘 | `game.rs::tick_mining`、`break_block_as_player`、`Items::mining_speed` / `correct_for_drops` | item個体の派生性能と条件付き応答を参照 |
| 耐久 | `game.rs::damage_held` とブロック破壊時の別処理 | vanillaの破棄前にModのdurability方針を選択。broken化を許す |
| 戦闘 | `game.rs::melee_damage`、`attack`、`attack_mob`、`game/combat.rs` | 攻撃前後の限定hookと状態異常の型を設計 |
| 独自Block Entity | `containers.rs::BlockEntity`、`block_entities.rs`、`save.rs`、`block_entity_nbt.rs` | 汎用Mod状態のvariant等を追加。既存の固定variantへTConstruct専用型を足さない |
| 機械tick | `game/block_entity_ticks.rs`、`game.rs::tick_world`、`game/levels.rs` | 既存tick領域・順序・dimension寿命へ接続 |
| menu | `menu.rs`、`container_screen.rs`、`game/stations.rs`、`game/lan_stations.rs` | 汎用slot layoutとサーバーrecipe transaction。既存UI widgetだけで代用しない |
| Mod通信 | `horizon-protocol`のCustomPayload、`lan/conn.rs`・`hub`、`game/remote.rs` | channelを実際のdispatchへ接続し、照合・データ同期を追加 |
| item描画 | `horizon-assets/src/item_model.rs`、`horizon-client/src/item_render.rs`、`game/item_draw.rs` | 汎用layer / tint / selectorを利用・拡張。原作loader名を本体へ直書きしない |
| 機械描画 | `block_entity_render.rs`、`block_entity_anim.rs` | 宣言的な液面・item・cuboid描画を後段で追加 |
| package検証 | `cargo-horizon-mod/src/check.rs`、`horizon-mods/src/testing.rs` | server guestとclient guestをそれぞれ試行。旧client専用probeに依存しない |

耐久消費は `damage_held` だけではありません。ブロック破壊時には `tool_damage_per_block` と `max_damage` を使う別経路があります。
HUDの耐久バーも `ItemInfo.max_damage` を直接参照しているため、ゲーム処理と表示の両方を個体の派生値へ対応させます。
`ItemInfo` を個体ごとに書き換える方式は、同じitem IDの他ツールへ影響するため採用しません。

## 5. API契約の設計案

### APIのバージョンとcomponent分割

現行package `horizon:mods@0.1.0` と0.1向けclient exportを変更して既存Modを破壊しないよう、サーバー契約は別version / worldで追加する案です。
正式な番号は実装時に決めます。現状の `api=0.3` や `0.4` が使えるとは記載しません。
manifestにhost APIとcomponent entrypointの対応を明示し、実importの権限検査を継続します。

最初のサーバーprobeはserver componentだけで構いませんが、完全版はサーバーのルール処理とクライアントの表示処理を分離できるpackage形式が望まれます。
たとえば将来の `server.wasm` / `client.wasm` と対応entrypointを検討します。現行CLIは `mod.wasm` 1個なので、これはpackage・manifest・SDK・CLI・runtime全体の追加作業です。
`side=both` を書くだけで役割分離や同期が完成することはありません。

### 5.1 データの読み込み

ホストはpackageの所有namespaceと決められた `data/` 範囲だけを読み取るresource APIを提供します。
初期の固定fixtureはRustの `include_str!` 等で埋め込めますが、最終的なdatapack拡張にはresource列挙・override順序・reload世代・エラー通知が必要です。
ファイルシステム全体へのアクセスを開く必要はありません。

Mod側はJSONを内部表現へ変換します。原作のmodule ID、型、既定値、tag参照、依存Mod条件を解決し、対応済み集合を作ります。
未対応moduleは読み飛ばさず、データのパスと理由を示してロード対象から外すか起動を失敗させます。
clientへ送るのは解決済みの必要データと世代・ハッシュとし、クライアントにサーバーとは別の条件で再解決させません。

### 5.2 ItemStack状態

初期案では、既存 `minecraft:custom_data` のcompound内に `tconstruct` 所有のpayloadを置き、最低限 `schema`、素材の順序付き配列、upgrade、persistent modifier data、broken状態を持たせます。
damageは既存フィールドとの対応を1か所で定義します。payloadとdamageに矛盾した二重の正本を作りません。
derived statsはキャッシュ扱いとし、定義・素材・Modifierの世代が変わったら再計算します。

WIT越しに渡す値はRustの `ItemStack` や可変参照ではなく、上限付きのDTOと、1イベント中だけ有効なハンドル・revisionです。
Modが更新できるのは所有payloadと許可された項目だけとし、別namespaceのデータや他人のslotを任意に書き換えるAPIにしません。
ホストがitem登録、個数、slot revision、所有者を検証して適用します。

`Clone`・split・merge・drop・container・item entity・playerdata・networkをすべて通ることを確認します。
payloadが異なる部品は誤ってstackされてはいけません。ツールは原則stack size 1とし、個体状態の同値判定を利用します。
未インストールModのデータや未知schemaは破棄せず保持し、使用停止・読取専用の扱いを定義します。

### 5.3 イベントと応答の適用

処理の形は **ホストがsnapshotを作る → Wasmへ問い合わせる → 応答を検証する → まとめて確定する** 方式を基本とします。
WasmへWorldの可変参照を渡したり、ホストmutation中に再入して同じslotを書き換えさせたりしません。
採掘等で即座に必要な値は同期query、素材消費・状態変更等は検証可能なtransactionとして区別します。

```text
入力: actor / dimension / 対象座標 / held snapshot / context / revision
応答: pass または handled、許可された値のoverride、状態更新案
確定: 対象とrevisionを再確認 → 制約検証 → 一度だけ適用 → 同期
```

有限でないfloat、範囲外の耐久・速度・item個数、無効なID、過大payloadを拒否します。
Modがtrapしたときは未確定の操作を棄却します。既存機械をvanillaの見た目だけで稼働継続させず、そのModの機械を停止して状態を保存します。

### 5.4 ゲーム進行・機械状態

サーバー全体の1tickと、各dimensionの1tickを分けます。メモリ上の機械はdimensionと座標をキーに管理し、`Parked` のswap・chunk unload・再ロードへ接続します。
Mod状態をすべてのdimensionで1回ずつ誤って初期化・多重実行しない設計が必要です。
専用サーバー・内蔵サーバー・in-frameの3経路で同じルールを通します。

初期のgeneric block entityは、所有Mod・型ID・schema・状態・slot・revision・tick要否・同期用派生値を保持する案です。
保存時は `save.rs` のdecode / encode双方とchunkのdirty化へ接続します。
未知Block Entityをraw保存する既存経路は救済に使えますが、raw NBTが残るだけではロード後のtick・操作・menuは動きません。

機械数が増えた段階ではModごとにbatch化します。vanilla tickerの順序を変えず、Mod機械の呼び出し順・途中の副作用が見える時点を明示します。
大きなmultiblockの探索はtick予算を使い切らないよう範囲制限と分割実行を設計します。

### 5.5 作業台と流体

Part Builderはpattern・材料・選択された部品・余剰材料の扱いを持ち、Tinker Stationは部品組み立て・交換・修理・Modifier適用を持ちます。
menuの結果slotはpreviewであり、取り出す瞬間に入力revision・recipe・持ち物・距離をサーバーが再検証し、消費と出力を同じtransactionで確定します。
shift-click、drag、swap、二人操作、破壊・切断の扱いを含めます。

Melter以降には液体の型と整数量・tank容量・温度・燃料・取引を追加します。
`simulate` と `commit`、面・容量・同時変更の整合性をホストが保証し、溶融・合金・鋳造のレシピ選択と進捗はMod側が決めます。
原作の量90等を他Modの慣習値へ変更せず、元recipeの単位と収支を保持します。

### 5.6 描画

性能・攻撃・耐久はサーバーの結果を正本とし、clientは表示に必要な状態だけを使います。
既存 `ItemModels` にはComposite / Condition / Select等とcomponent patchを持つ `StackView` があるため、まず既存のlayer機構で表現できる範囲を検証します。
不足する場合に汎用sprite・material tint・状態selectorを追加し、TConstruct側で原作の `tconstruct:tool` modelを宣言形式へ変換します。
大量の素材組み合わせを別item IDへ展開したり、毎frame全ツールのWasm呼び出しでモデルを作り直したりしません。
キャッシュキーはtool種別、部品素材、表示Modifier、broken、resource世代等から作り、無制限に増えないよう上限を設けます。

液面・鋳造中item・流路は、ホストが扱えるcuboid・item transform・透明度等の宣言へ落とします。
まず静的な最小表現、次に液量とcooling進捗に同期する描画へ進めます。任意のGPU handleやshaderをWasmへ公開する必要はありません。

### 5.7 通信と互換性

既存CustomPayloadのcodecだけではMod通信は成立しません。configuration / playの受送信dispatchに、protocol version、Mod ID / version、内容ハッシュ、registry対応を実装します。
内蔵サーバーも同じ検証を行います。クライアントが不足するcontentを持つ場合は、機械へ接続する前に明確な理由で拒否します。
自動ダウンロードは最初の実装範囲に含めず、両端へ同じpackageを手動配置します。

TConstructの新item・blockを持つworldでは、vanillaクライアントの接続互換を目標にしません。
反対に、Mod無効のゲームやvanillaサーバーへ接続する経路で、新しいregistryやpayloadを勝手に要求しないようにします。
大きな同期データは上限・分割・順序を定義し、NBTやcomponentは受信しただけで信頼しません。

## 6. 最初の実装範囲をデータまで固定する

### 初期コア

候補はwood / flintのhead・handle・binding、pickaxeです。
wood・flintにはammo向けの別traitもありますが、初期のpickaxeに必要なstatとtraitだけを解決します。

| データ | 初期に必要なもの |
| --- | --- |
| material | definition、対象stat、default / perStat trait、表示名・色 |
| pickaxe | part_stats、default_materials、base_stats、modifier_slots、traits、tool_actions、is_effective、box_aoe、fixed_material_name |
| cultivated | repair係数 |
| jagged | conditional_melee_damage、conditional_mining_speed、式評価、lost durability等の変数 |
| pierce | stat_boostとweapon_mob_effect、`tconstruct:pierce`状態異常 |
| Part Builder | head cost=2、handle cost=1、binding cost=1。pattern条件と材料換算・残量を含む |
| Station | tool_building、修理、部品交換。初期upgrade追加の範囲は別に固定 |

tool定義にmoduleが存在しても、初期使用で発火しない機能は「未対応・未発火」を区別します。
たとえばAOEを不要として黙って捨てず、expansion未導入の初期範囲と、拡張が有効になった時点の非対応診断を明記します。
ランダム既定素材・tagの有効集合・Modifier合成規則まで含め、選んだ範囲の依存一覧を実装開始時に機械可読で出力します。

### 数値計算の合わせ方

原作 `FloatToolStat` の集約は `(base + add) * (1 + percent) * multiply` です。
`update` はbaseを0にしながら加算し、`add`・`percent`・`multiply` は別段階に蓄積します。
materialのstat適用、tool module、trait・upgrade、volatile data、clamp・整数化までの順番を保存します。
floatはJavaのfloatに相当する箇所を `f32` として扱い、途中値を含むfixtureで確認します。式評価の型やsqrt等は原作の実装に合わせて別途確定します。

検証用の組み合わせは、木のみ、flint head＋木handle/binding、同一素材の重複trait、最大耐久・破損・修理境界です。
原作期待値をまだ計測していないため、この文書に最終性能値を推定で記載していません。

参照: [FloatToolStat](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/library/tools/stat/FloatToolStat.java)、[部品recipe](https://github.com/SlimeKnights/TinkersConstruct/tree/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/generated/resources/data/tconstruct/recipes/tools/parts/builder)、[状態異常登録](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/shared/TinkerEffects.java)。

## 7. 実装順序と完了条件

各行は作業のまとまりです。広い変更は本体の約500行以内の小さいPRという方針に合わせて、さらに分割します。

| 順 | 本体側 | Mod側 | 完了条件 |
| --- | --- | --- | --- |
| 0 | 保存・通信の既存往復をfixture化 | 対象データ・出典・schema・trait依存を固定 | 追加payloadがどの経路で残り、どこで欠けるか再現できる |
| 1 | まだ変更不要 | 素材loader、stats、ToolState、Modifierの小集合 | Java由来fixtureと一致。壊れた入力を診断できる |
| 2 | role別server runtime、SDK、probe、integratedへのdev package引き渡し | 無害なserver lifecycle probe | in-frame / integrated / dedicatedで必要な回数だけ動く。旧client Modも動く |
| 3 | custom dataの送信、item hookとdurability方針 | tool state adapter、pickaxeの採掘・攻撃・broken | 別個体の性能を混同せず、破損しても消えない。保存・同期で保持 |
| 4 | generic Block Entityとmenu、revision付きtransaction | Part Builder / Tinker Stationのrecipe | 部品作成・組み立て・修理・交換が二重消費や複製なく成立 |
| 5 | layer / tintと同期stateの描画 | model変換、tooltip、trait表示 | インベントリ・手持ち・dropで部品素材とbroken表示が一致 |
| 6 | fluid / tank / machine tick / 液体描画 | Melter → Faucet → Casting Table | 燃料・入力・型・液体・cooling・出力の収支が一致 |
| 7 | bounded multiblock query・関連同期 | Smeltery・合金・他ツール等を順次追加 | 構造変更・chunk境界・保存復帰・複数人・負荷を確認 |

段階2〜5が最初の遊べる体験です。UI表示だけ・vanilla pickaxeの名前変更だけでは、その完了条件を満たしません。
段階3の試作ではテスト用生成手段でtool個体を用意して先に動作を検証し、段階4で正規の制作経路を接続できます。

## 8. 検証と開発環境

確認したローカル環境はRust / Cargo 1.99.1、Windows MSVCです。
インストール済みtargetは `x86_64-pc-windows-msvc` と `wasm32-wasip2` で、必要な **`wasm32-unknown-unknown` は未導入**です。
Horizonの `.local/horizon-client/26.3` と `.local/minecraft/26.3` は存在しませんでした。
そのため、現状だけでゲーム内検証・vanillaとの完全なparity検証が実施可能とは判断していません。

GitHub CLIは見つかりましたが、forkの既存PR確認は認証不足で失敗しました。
調査自体はローカルソースから進められます。実装開始時に、利用可能な接続または認証済みCLIで重複するPRを確認します。今回はPR作成・投稿・remote変更を行っていません。

### 実装時の検証層

| 層 | 検証内容 |
| --- | --- |
| Modのpure Rust | loader、数値集約、trait順序、修理、broken、serialization、未対応ID |
| host契約 | WIT互換、権限、role、fuel・メモリ上限、trap時の未確定操作取消 |
| item往復 | save / load、menu、drop、nested container、packet encode / decode、同値判定 |
| ゲームプレイ | 採掘・攻撃・耐久・表示。非Modアイテムの結果が変わらないこと |
| menu | pickup / quick move / drag / swap / output満杯 / 二人操作 / 切断 / 破壊 |
| 機械 | tick順、freeze・pause、dimension swap、chunk unload、構造変更、液体収支 |
| 複数実行経路 | in-frame、既定integrated、LAN client、dedicated、Mod不一致の拒否 |
| 原作比較 | 固定した1.20.1 TConstruct＋Mantleで生成したfixtureとRustを比較 |

本体では変更したcrateの適切なtestと、`tools/parity.py --changed-files ... --list` による影響範囲を使います。
`tools/pr_ready.py --quick --base origin/main` はforkのmainを基準にする場合の候補です。上流へ提案する場合は、存在を確認した上流refへ基準を変えます。
このcheckoutにはoriginしか確認していないため、存在しない `upstream/main` を前提にコマンドを実行しません。
新しいMod用試験はゲームなしのsandbox試験と、専用のゲームfixtureの両方を追加します。既存TOMLのclient snapshot試験だけでサーバー機械を検証済みとしません。

原作のJava / Gradleは開発用oracle環境だけに使い、プレイヤー実行環境へ持ち込みません。
Minecraft由来のjar・抽出物・準備済みresourceは公開せず、本体の `.local/` 等に保持します。
ゲーム起動時には本体AGENTSの規約に従い、`HORIZON_HOME`、`HORIZON_LAUNCHER_HOME`、`XDG_DATA_HOME`、`XDG_CONFIG_HOME` を専用の一時ディレクトリへ分離し、self-update無効・audio無効で検証します。

## 9. 初回実装で行わないこと

初回は全Modifier、全素材、全worldgen、全武器・防具、book全形式、JEIや他Modの互換、原作1.20.1ワールドの自動変換までを含めません。
それらは最終的な移植範囲から除外する決定ではなく、コアとhost APIを確認した後の段階です。
Horizon本体へTConstructのレシピ名・素材一覧・trait計算を埋め込みません。
ゲーム描画ができることとサーバー処理が正しいことを別々に確認します。

## 10. 次に着手できる具体的な作業

最初の作業は **対象データの取り込み・依存一覧・数値fixture・ItemStack保存と通信の試験** です。
これでModコアの仕様とホストの欠落経路を先に固定できます。その後、server runtimeの小さなprobeを通し、item hookへ進めます。

残る設計判断はserver ABIの正式version、dual componentのpackage形式、generic menuの最小contract、動的な状態異常の登録方式です。
その判断に必要な箇所と受け入れ条件は上記に示しましたが、具体的なRust型やWITをコンパイルして成立確認した段階ではありません。
