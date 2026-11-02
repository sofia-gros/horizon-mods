# Materials and You

This in-progress guide uses the original English Materials and You data and images from TConstruct revision `f4e77d43f39ba1047bf93ae816b7e1beaab514c2`. Craft it from a book and a pattern. The source-config-controlled starting-book advancement uses saved per-player progress.

Open the original cover and turn two-page spreads with the arrows or book controls. Chapter and page indexes link to the source text, illustrations, recipes and generated material and tool pages. Recipes cycle through loaded tag candidates and show their result count. Material pages show the part, its stats and its traits. A LAN guest requests the host-resolved pages. The Basic Armor section appears when its source item tag is populated.

The following differences from Mantle's original reader remain:

- Illustrated material panels use the source colors, part icons, stats, traits and available material actions. Long material stat groups continue onto additional pages; exact parity with the original panel and every material is not established. Some tool properties and long image descriptions continue onto later pages, and long index entries use shortened excerpts.
- Tool and part preview hover text uses the existing item tooltip renderer. The earlier tested tree supports source modifier showcase tooltips. Exact page-element placement and image scaling have not been verified for every source page.
- Material actions requiring unregistered fluid buckets or instrument-specific item components are omitted rather than represented with an incorrect item. Their source display remains incomplete.
- Part Chest, Tinker's Chest and later guide books are not registered here; their source recipe diagrams can show an empty output icon. This guide's source text can describe other features that this port does not yet implement.
- Other languages and the remaining guides are future work.

The locally tested runtime corresponds to Git tree `f43c48bf04556085fa16967fdba1eee222ebbb8e` over commit `87d61cb5141511adbc5e8fe158e5cf4a16ccd692`. Linux representative checks covered a reset starting-book reward and reload without duplication, book-and-pattern crafting, cover, links on both pages, history back, a recipe link and tag-candidate cycling, source illustrations and tool previews, and long text with bold and italic styling. The source modifier icon tooltip was checked on hover. A temporary datapack populated the armor tag to inspect Traveler's Goggles; a separate host-only fixture verified that the guest received the extra armor chapter. Host and guest also showed the same styled page. The fixtures were removed after testing. These are not exhaustive checks of every page or platform.

The later local runtime tree `7eef3221b95cae12875527d7b94c1ea26806e567` passed the official Mixins transformation (revision `ddd9700b21997c95d95ac6ee5ecafe3830cf622a`) and the Linux target build. In the existing test world, the Wood panel showed its source color, head/handle/binding stats and Cultivated trait; the wooden part and Broad Axe hover names appeared. Wrapped values stayed with their labels after a layout fix. The same Wood spread was visible to the LAN host and guest, and cover, both chapter links, history back and representative long further-reading text were checked. The Puny Smelting recipe showed its registered inputs and the expected blank unregistered result. These checks use representative pages, not every material or model. The new test screenshots are local only and have not been published.

The following figures are from the earlier two-page runtime `87d61cb`, not the new local tree.

![The original cover in the test world](materials-and-you-cover-test.png)
![A source crafting diagram in the test world](materials-and-you-recipe-test.png)

The crafting-grid texture is from Mantle 1.20 revision `17c287ad52f7b10154e07fa7570dc7bccd74ca2b`, under the MIT license in `third_party/mantle/LICENSE`.

The pinned guide JSON and images are ordinary files under `content/assets/tconstruct/book/`. The content overlay supplies the same paths to the reader and LAN host; resource preparation copies the original guide images into the GUI atlas. There is no separate guide ZIP to install. The `.hmod` content package and `.hnmod` native package remain separate, and this directory change has not yet been checked in a packaged game.
