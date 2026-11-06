# Latest official Template: source-only compatibility candidate

This group targets official Horizon `f4dcf8a2fb4a68b2d28422f4d34d2be2970e26bd`.
It does not alter the old successful common source, its package, any compatibility
pin, normal manifest, public API or consumer activation. No patch is applied and
no Rust, metadata, planner or native build is run. Six Rust cases are prepared,
not passed tests. The source-only audit is in `official-audit.json`.

## What changed and what did not

The latest native `crafting::Template` adds `components: Option<Tag>` containing
the result's raw patch before decoding, including explicit removals and zeroes.
`parse` retains it. `over` applies the input patch followed by this raw result
patch, with removals replacing values, and preserves the existing runtime Wire
and bundle selection separately. `create()` remains byte-identical to the old
version and still returns the decoded fixed stack. Reconstructing a template
from that stack cannot recover every raw patch: explicit damage zero is one
concrete counterexample.

The actual common archive `4f13a6e2…` has 147 members. Its only Template-related
files are the matching adapter and host-independent `TemplateFacts` declaration.
There is no native Template construction literal in the ordinary package.
The adapter borrows fields and calls native `create`; it neither copies a raw
patch into another Template nor performs transmutation. Its `has_components`
currently means decoded `extra.is_some()`, not raw-patch presence. No new public
field or speculative raw-patch authority is added here.

RecipeBook preview/presentation/index operate on native Entry/ItemStack/Known,
not Template. The entire latest native `recipe_book.rs` is byte-identical to the
old version. This limited observation does not prove a latest full-package build
or composition, and the old mandatory source/target guards still require review.

## Two proposed fixture-only changes

`fixture-literals.patch` adds `components: None` to exactly two excluded factories:

- `tests/native/crafting_match.rs` builds a synthetic template from a supplied
  full ItemStack and its existing checks exercise `create`, fixed shaped/shapeless
  matching and component facts. It has no raw parsed result patch to retain.
- `tests/native/recipe_placement.rs` builds a synthetic plain output with no
  components; placement checks do not call Template::over.

For those exact old oracles, None retains the intended construction. It is **not**
a general production recipe migration. The old fixtures, tools, pins and oracle
bodies remain unchanged; the proposed two-line patch is unapplied. The latest
Template declaration would make these old literals incomplete if those fixtures
were prepared against the new native structure. No production compile failure
has been demonstrated by this source audit.

## Minimal actual-native test contract

`template_tests.rs` expects `native-template.rs` to contain the exact source
region from `#[derive(Clone, Debug)]\npub struct Template` through the end of
`json_tag`, immediately before `#[derive(Clone, Debug)]\npub enum Recipe`.
This includes real parse/create/over, assembly_components and JSON conversion,
not a mocked implementation. Use real latest official `horizon-items` and
`horizon-world` dependencies plus serde_json, the latest official lock, and exact
native path identities in a later separately reviewed runner. No new workflow,
generated Cargo project or implicit metadata operation is part of this group.

The six cases check absent versus empty raw patch, explicit zero versus lossy
reconstruction, removal precedence and unchanged input, native clone preservation,
the two old create-only factory oracles, and transient map postprocess versus saved
components. A later pass would cover those finite native cases only, not automatic
craft output/remainder authority, arbitrary saved/opaque components or gameplay.

## Other scoped latest changes

item_sync.rs, item_defaults.rs and held-name titles.rs are byte-identical. The
guest codec extraction region is unchanged, while the whole guest file gained
handover fields. The old whole-file exclusivity hash must not simply be waived.
item_data adds saved-default canonicalization and broader numeric damage decoding;
protocol item decoding expands version-specific EitherHolder handling. These are
real dependency changes requiring latest codec proof even though the tested name
entry regions stayed the same. Default-name payload ownership is still missing.
Tooltip container lines now use native indexed Java formatting and drawing adds a
pre-render hook/bundle visibility logic. No blanket translation or rendering patch
is proposed. The old successful codec32 proof remains explicitly old-official.
