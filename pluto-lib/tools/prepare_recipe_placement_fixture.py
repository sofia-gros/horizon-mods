#!/usr/bin/env python3
"""Exact official placement bodies with modeled Game/menu mutation boundaries.

No host edits. This tests veto ordering, not live gameplay or Mixins expansion.
"""
import argparse
import json
from pathlib import Path
import shutil
import subprocess
import tomllib
import prepare_advancement_projection_fixture as common
from prepare_recipe_preview_fixture import fragment

ROOT = Path(__file__).resolve().parents[1]
LOCAL = {("pluto-recipe-placement-tests", "0.0.0", None, None), ("pluto-inventory-api", "0.1.0", None, None)}
REFERENCE = "8c56bb52b6b77fae6115e894af9e369a5c3f9e08"

def verify_routing(host):
    def ordered(source, *markers):
        positions = [source.index(marker) for marker in markers]
        if positions != sorted(positions): raise ValueError("Native placement boundary moved after effects")
    game = (host / "crates/horizon-client/src/game/recipe_book.rs").read_text()
    local = fragment(game, "    fn place_recipe(&mut self, book_type: BookType, entry_index: usize, use_max: bool) {", "    /// The container changed under the book")
    ordered(local, "if self.remote.is_some()", "self.send(", "self.menu_session(", ".placement(&entry.id)", "menu.place_recipe(")
    menu = fragment((host / "crates/horizon-client/src/menu/place.rs").read_text(), "    pub fn place_recipe(\n", "    /// `recipeMatches`")
    for effect in ("self.test_clear_grid", "self.stacked_contents", "self.place(", "self.clear_grid", "self.recompute"):
        ordered(menu, "env.recipes.placement(id)", effect)
    lan = fragment((host / "crates/horizon-client/src/game/lan_crafting.rs").read_text(), "    pub(super) fn lan_place_recipe(\n", "    /// The guest opened a crafting table")
    ordered(lan, "recipe_of_display(p.recipe)", "!g.recipe_book.knows(&id)", ".placement(&id)", "self.native_player_recipe_book_click", "self.guest_menu_op")
    if lan.count("menu.place_recipe(env, &id, p.use_max, creative, None)") != 2: raise ValueError("Unreviewed LAN placement route")
    if game.count("self.place_recipe(BookType::Crafting, index, all);") != 1 or game.count("self.place_recipe(book_type, entry_index, use_max);") != 1:
        raise ValueError("Unreviewed mod/click dispatch route")
    book = (host / "crates/horizon-client/src/recipe_book.rs").read_text()
    if '.filter_map(|e| Some((e.id.clone(), recipes.placement(&e.id)?)))' not in book:
        raise ValueError("Book craftable index no longer uses canonical placement")
    return ["local UI before remote send/menu dispatch", "native menu before clear/move/recompute", "LAN authority/knowledge then canonical placement before both guest menu paths", "ordinary and native-mod click dispatch", "book craftable projection uses canonical placement"]

def verify(output, expected_name="pluto-recipe-placement-tests"):
    evidence = json.loads((output / "fixture-sources.json").read_text())
    host = Path(evidence["official_source"])
    common.clean_official(host, evidence["official_revision"])
    if verify_routing(host) != evidence["routing_assertions"]: raise ValueError("Routing assertion drift")
    for p, sha in evidence["official_sha256"].items():
        if common.digest((host / p).read_bytes()) != sha: raise ValueError("Official source drift")
    for p, sha in evidence["files_sha256"].items():
        if common.digest((output / p).read_bytes()) != sha: raise ValueError("Fixture source drift: " + p)
    for part in evidence["fragments"]:
        exact = fragment((host / part["source"]).read_text(), part["start"], part["end"])
        if common.digest(exact.encode()) != part["sha256"]: raise ValueError("Native fragment drift")
        inserted = exact.replace(part.get("rename_from", "\0"), part.get("rename_to", "\0"), 1)
        if inserted not in (output / part["destination"]).read_text(): raise ValueError("Native body is not exact")
    official = {common.identity(p) for p in tomllib.loads((host / "Cargo.lock").read_text())["package"]}
    resolved = {common.identity(p) for p in tomllib.loads((output / "Cargo.lock").read_text())["package"]}
    local = {(expected_name, "0.0.0", None, None), ("pluto-inventory-api", "0.1.0", None, None)}
    if resolved - official - local or not local.issubset(resolved): raise ValueError("Dependency identity drift")
    paths = {expected_name: output / "Cargo.toml", "pluto-inventory-api": output / "inventory-api/Cargo.toml"}
    seen = set()
    for package in common.metadata(output, True)["packages"]:
        path = Path(package["manifest_path"]).resolve()
        if package["name"] in paths:
            if package["source"] is not None or path != paths[package["name"]].resolve(): raise ValueError("Local package drift")
            seen.add(package["name"])
        elif package["source"] is None and not path.is_relative_to(host): raise ValueError("Foreign path dependency")
    if seen != set(paths): raise ValueError("Missing package")
    return evidence

def prepare(host, output):
    host = host.resolve()
    native = ROOT / "mixins/pluto_native_creative"
    pin = tomllib.loads((native / "horizon-mixin.toml").read_text())["compatibility"]["horizon_revision"]
    common.clean_official(host, pin)
    if output.exists(): raise ValueError("Output must be new")
    output.mkdir(parents=True)
    shutil.copytree(native / "inventory-api", output / "inventory-api", ignore=shutil.ignore_patterns("target", "Cargo.lock"))
    files = {"recipe-placement.rs": "placement.rs", "recipe-placement-definition.rs": "definition.rs", "recipe-placement-attempt.rs": "attempt.rs", "recipe-placement-click.rs": "click.rs", "recipe-book-preview.rs": "preview.rs", "stack-storage.rs": "stack-storage.rs"}
    for src, dst in files.items(): shutil.copyfile(native / src, output / dst)
    shutil.copyfile(ROOT / "tests/native/recipe_placement.rs", output / "lib.rs")
    policy = ROOT.parent / "tconstruct/tests/rust/recipe-placement-policy"
    for name in ("callback.rs", "reference_marked.rs"): shutil.copyfile(policy / name, output / name)
    # Compare the reference's exact function bytes with the authorized MOD source.
    original = subprocess.check_output(["git", "-C", str(ROOT.parent), "show", REFERENCE + ":tconstruct/mixins/tconstruct/source-fake-material-book.rs"], text=True)
    marked = original[original.index("pub fn marked("):original.index("// Only the generator")]
    marker = next(line for line in original.splitlines() if line.startswith("const MARKER:"))
    if marked not in (output / "reference_marked.rs").read_text() or marker not in (output / "reference_marked.rs").read_text(): raise ValueError("Consumer policy reference drift")
    records = []
    def take(file, start, end, dest, rename=None):
        text = fragment((host / file).read_text(), start, end)
        record = dict(source=file, start=start, end=end, destination=dest, sha256=common.digest(text.encode()))
        if rename:
            record.update(rename_from=rename[0], rename_to=rename[1]); text=text.replace(*rename, 1)
        records.append(record); return text
    crafting = "crates/horizon-client/src/crafting.rs"; book = "crates/horizon-client/src/recipe_book.rs"
    menu = "crates/horizon-client/src/menu/place.rs"; game = "crates/horizon-client/src/game/recipe_book.rs"
    c = 'use std::{collections::HashSet,sync::Arc};\nuse serde_json::Value;\nuse horizon_items::{items::ItemStack,item_data::StackData};\nuse horizon_items::firework::FireworkShape;\n'
    for start, end in [("#[derive(Clone, Debug)]\npub struct Ingredient", "impl Ingredient {"), ("#[derive(Clone, Debug, Default)]\npub struct Placement", "/// A recipe result:"), ("#[derive(Clone, Debug)]\npub struct Template", "/// `ItemStackTemplate.CODEC` count:"), ("#[derive(Clone, Debug)]\npub enum Recipe", "impl Special {\n    /// The recipe-book displays"), ("#[derive(Clone, Debug)]\npub struct Cooking", "/// A recipe of a type of a mod's own")]:
        c += take(crafting, start, end, "crafting.rs")
    c += '#[derive(Default)] pub struct Recipes {pub cooking:Vec<Cooking>,pub crafting:Vec<(String,Recipe)>}\nimpl Recipes {\n'
    c += take(crafting, "    pub fn placement(&self, id: &str) -> Option<Placement> {", "    /// `Recipe.getRemainingItems`", "crafting.rs", ("pub fn placement(", "pub fn native_placement("))
    c += 'pub fn placement(&self,id:&str)->Option<Placement>{match pluto_recipe_placement_definition(self,id){std::ops::ControlFlow::Break(v)=>v,std::ops::ControlFlow::Continue(())=>self.native_placement(id)}}\n}\ninclude!("definition.rs");\n#[path="placement.rs"] pub mod pluto_placement;\n'
    (output / "crafting.rs").write_text(c)
    b = 'use crate::crafting::{Ingredient,Placement,Recipes};\nuse horizon_items::items::ItemStack;\nuse std::collections::HashMap;\n'
    for start, end in [("#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]\npub enum BookType", "impl BookType {"), ("#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]\npub enum Category", "impl Category {"), ("#[derive(Clone, Debug)]\npub enum Display", "/// A group of recipes shown as one button"), ("#[derive(Default)]\npub struct Book", "pub(crate) fn crafting_category")]:
        b += take(book,start,end,"recipe_book.rs")
    b += 'pub fn native_book_placements(entries:&[Entry],recipes:&Recipes)->HashMap<String,Placement>{\n'
    b += take(book, "        let placements = entries\n", "        Self {\n            entries,\n            placements,", "recipe_book.rs")
    b += 'placements\n}\n'
    (output / "recipe_book.rs").write_text(b)
    m = 'use crate::{crafting::{Ingredient,Placement},items::ItemStack};\nuse std::{collections::HashMap,sync::Arc};\n'
    m += take(menu,"#[derive(Clone, Debug, Default)]\npub struct Stacked", "impl Menu {", "native_menu.rs")
    m += 'impl Menu {\n'
    m += take(menu,"    pub fn place_recipe(\n", "    /// `recipeMatches`", "native_menu.rs")
    m += '}\n'
    (output / "native_menu.rs").write_text(m)
    g = 'impl Game<\'_> {\n'
    g += take(game,"    fn place_recipe(&mut self, book_type: BookType, entry_index: usize, use_max: bool) {", "    /// The container changed under the book", "native_game.rs", ("fn place_recipe(","fn native_place_recipe("))
    g += 'fn place_recipe(&mut self,book:BookType,index:usize,all:bool){if let std::ops::ControlFlow::Continue(())=pluto_recipe_placement_click(self,book,index){self.native_place_recipe(book,index,all)}}\n}\n'
    (output / "native_game.rs").write_text(g)
    manifest='[package]\nname="pluto-recipe-placement-tests"\nversion="0.0.0"\nedition="2024"\n[workspace]\n[lib]\npath="lib.rs"\n[dependencies]\nserde_json="1"\npluto-inventory-api={path="inventory-api"}\n'
    for name in ("horizon-items","horizon-world","horizon-core","horizon-protocol"):
        manifest += name+'={path='+json.dumps(str(host / "crates" / name))+'}\n'
    manifest += '\n[patch.crates-io]\n'
    for name, patch in tomllib.loads((host / "Cargo.toml").read_text()).get("patch",{}).get("crates-io",{}).items():
        if set(patch)!={"path"}: raise ValueError("Unreviewed official patch")
        path=(host/patch["path"]).resolve()
        if not path.is_relative_to(host): raise ValueError("Escaping official patch")
        manifest += json.dumps(name)+'={path='+json.dumps(str(path))+'}\n'
    (output / "Cargo.toml").write_text(manifest)
    evidence=dict(official_source=str(host), official_revision=pin, consumer_reference=REFERENCE,
        consumer_marked_sha256=common.digest(marked.encode()), fragments=records, routing_assertions=verify_routing(host),
        official_sha256={p:common.digest((host/p).read_bytes()) for p in (crafting,book,menu,game,"Cargo.lock","crates/horizon-client/src/game/lan_crafting.rs")},
        files_sha256={str(p.relative_to(output)):common.digest(p.read_bytes()) for p in output.rglob('*') if p.is_file()},
        scope="Exact native placement/menu/book bodies and ItemStack; outer recipes/menu/Game mutation and network operations modeled. HEAD wrappers modeled, actual Mixins composition remains a separate package gate.")
    (output / "Cargo.lock").write_bytes((host / "Cargo.lock").read_bytes()); common.metadata(output, False)
    (output / "fixture-sources.json").write_text(json.dumps(evidence,indent=2)+'\n'); verify(output); print(output)

if __name__ == '__main__':
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--horizon-source',type=Path);parser.add_argument('--output',type=Path,required=True);parser.add_argument('--verify-lock',action='store_true');args=parser.parse_args()
    if args.verify_lock: verify(args.output);print('Exact native bodies and locked dependency identities verified')
    elif args.horizon_source is None: parser.error('--horizon-source required')
    else: prepare(args.horizon_source,args.output)
