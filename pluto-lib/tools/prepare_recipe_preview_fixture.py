#!/usr/bin/env python3
"""Prepare bounded preview checks with exact official declarations/layout code.

No host edits or downloads. The Game/menu boundary is modeled explicitly; native
Entry/Ingredient/layout declarations, ItemStack and stack equality are real.
"""
import argparse
import json
from pathlib import Path
import shutil
import tomllib
import prepare_advancement_projection_fixture as common

ROOT=Path(__file__).resolve().parents[1]
LOCAL={("pluto-recipe-preview-tests","0.0.0",None,None),("pluto-inventory-api","0.1.0",None,None)}


def fragment(source,start,end):
    if source.count(start)!=1 or source.count(end)!=1:
        raise ValueError("Official source selectors are no longer unique")
    return source[source.index(start):source.index(end,source.index(start))]


def verify(output):
    evidence=json.loads((output/"fixture-sources.json").read_text());host=Path(evidence["official_source"])
    common.clean_official(host,evidence["official_revision"])
    for path,sha in evidence["official_sha256"].items():
        if common.digest((host/path).read_bytes())!=sha:raise ValueError("Official source drifted: "+path)
    for path,sha in evidence["files_sha256"].items():
        if common.digest((output/path).read_bytes())!=sha:raise ValueError("Fixture source drifted: "+path)
    for part in evidence["fragments"]:
        actual=fragment((host/part["source"]).read_text(),part["start"],part["end"])
        if common.digest(actual.encode())!=part["sha256"] or actual not in (output/part["destination"]).read_text():
            raise ValueError("Selected official fragment is not byte exact")
    official={common.identity(p) for p in tomllib.loads((host/"Cargo.lock").read_text())["package"]}
    resolved={common.identity(p) for p in tomllib.loads((output/"Cargo.lock").read_text())["package"]}
    if resolved-official-LOCAL or not LOCAL.issubset(resolved):raise ValueError("Fixture dependency identity drift")
    required={"pluto-recipe-preview-tests":output/"Cargo.toml","pluto-inventory-api":output/"inventory-api/Cargo.toml"};found=set()
    for p in common.metadata(output,locked=True)["packages"]:
        path=Path(p["manifest_path"]).resolve()
        if p["name"] in required:
            if p["source"] is not None or path!=required[p["name"]].resolve():raise ValueError("Local package source drift")
            found.add(p["name"])
        elif p["source"] is None and not path.is_relative_to(host):raise ValueError("Unexpected local dependency")
    if found!=set(required):raise ValueError("Missing local package")
    return evidence


def prepare(host,output):
    host=host.resolve();pin=tomllib.loads((ROOT/"mixins/pluto_native_creative/horizon-mixin.toml").read_text())["compatibility"]["horizon_revision"]
    common.clean_official(host,pin)
    if output.exists():raise ValueError("Output must be new")
    output.mkdir(parents=True);native=ROOT/"mixins/pluto_native_creative"
    shutil.copytree(native/"inventory-api",output/"inventory-api",ignore=shutil.ignore_patterns("target","Cargo.lock"))
    for source,dest in [(native/"recipe-book-preview.rs","preview.rs"),(native/"recipe-book-preview-cells.rs","preview-cells.rs"),(native/"stack-storage.rs","stack-storage.rs"),(ROOT/"tests/native/recipe_preview.rs","lib.rs")]:shutil.copyfile(source,output/dest)
    records=[]
    def take(file,start,end,dest):
        text=fragment((host/file).read_text(),start,end);records.append(dict(source=file,start=start,end=end,destination=dest,sha256=common.digest(text.encode())));return text
    crafting="crates/horizon-client/src/crafting.rs";book="crates/horizon-client/src/recipe_book.rs"
    (output/"crafting.rs").write_text('use std::collections::HashSet;\nuse serde_json::Value;\n'+take(crafting,'#[derive(Clone, Debug)]\npub struct Ingredient','impl Ingredient {','crafting.rs')+take(crafting,'#[derive(Clone, Debug, Default)]\npub struct Placement','impl Placement {','crafting.rs'))
    parts=[take(book,'#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]\npub enum BookType','impl BookType {','recipe_book.rs'),
        take(book,'#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]\npub enum Category','impl Category {','recipe_book.rs'),
        take(book,'#[derive(Clone, Debug)]\npub enum Display','/// A group of recipes shown as one button','recipe_book.rs'),
        take(book,'#[derive(Default)]\npub struct Book','pub(crate) fn crafting_category','recipe_book.rs')]
    items_of=take(book,'    pub fn items_of(&self, ingredient: &Ingredient) -> Vec<String> {','    /// `RecipeBookComponent.fillGhostRecipe`','recipe_book.rs')
    (output/"recipe_book.rs").write_text('use crate::crafting::{Ingredient,Placement};\nuse horizon_items::items::ItemStack;\nuse std::collections::HashMap;\n'+''.join(parts)+'impl Book {\n'+items_of+'}\n#[path="preview-cells.rs"] pub mod pluto_preview_cells;\n')
    (output/"Cargo.toml").write_text('[package]\nname="pluto-recipe-preview-tests"\nversion="0.0.0"\nedition="2024"\n[workspace]\n[lib]\npath="lib.rs"\n[dependencies]\nserde_json="1"\npluto-inventory-api={path="inventory-api"}\n'+''.join(name+'={path='+json.dumps(str(host/"crates"/name))+'}\n' for name in ("horizon-items","horizon-world")))
    patches=tomllib.loads((host/"Cargo.toml").read_text()).get("patch",{}).get("crates-io",{})
    with (output/"Cargo.toml").open('a') as f:
        f.write('\n[patch.crates-io]\n')
        for name,entry in patches.items():
            if set(entry)!={"path"}:raise ValueError("Unreviewed official patch")
            path=(host/entry["path"]).resolve()
            if not path.is_relative_to(host):raise ValueError("Official patch escapes checkout")
            f.write(json.dumps(name)+'={path='+json.dumps(str(path))+'}\n')
    evidence={"official_source":str(host),"official_revision":pin,"official_sha256":{p:common.digest((host/p).read_bytes()) for p in (crafting,book,"Cargo.lock")},"fragments":records,"files_sha256":{str(p.relative_to(output)):common.digest(p.read_bytes()) for p in output.rglob('*') if p.is_file()},"scope":"Exact native declarations/layout, real ItemStack and production adapters; outer Game and ghost-call recording modeled, not game/UI/placement proof."}
    (output/"Cargo.lock").write_bytes((host/"Cargo.lock").read_bytes());common.metadata(output,locked=False)
    (output/"fixture-sources.json").write_text(json.dumps(evidence,indent=2)+'\n');verify(output);print(output.resolve())


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--horizon-source',type=Path);parser.add_argument('--output',type=Path,required=True);parser.add_argument('--verify-lock',action='store_true');args=parser.parse_args()
    if args.verify_lock:verify(args.output);print('Exact official fragments, helper/API hashes and dependency identities verified')
    elif args.horizon_source is None:parser.error('--horizon-source is required')
    else:prepare(args.horizon_source,args.output)
