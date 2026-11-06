#!/usr/bin/env python3
"""Prepare explicit pair staging over exact native Input/items/component codecs."""
import argparse,json,shutil,subprocess
from pathlib import Path
import prepare_crafting_match_fixture as matching
import prepare_recipe_placement_fixture as base
import prepare_advancement_projection_fixture as common
ROOT=Path(__file__).resolve().parents[1]
NAME='pluto-crafting-products-tests'
def routing(host):
    # These source assertions explain the deferred automatic integration. They
    # are not execution of Menu/crafter consumption.
    menu=(host/'crates/horizon-client/src/menu.rs').read_text()
    limit=menu[menu.index('        let patched = stack.data().and_then(|d| {'):menu.index('        patched.unwrap_or_else(|| env.items.get(&stack.item).map_or(64, |i| i.max_stack))')]
    # Ignore indentation only; the native limit operations must be unchanged.
    adapter=(ROOT/'mixins/pluto_native_creative/crafting-products.rs').read_text()
    if ''.join(limit.split()) not in ''.join(adapter.split()):raise ValueError('Native stack-limit calculation drift')
    take=menu[menu.index('    fn safe_take('):menu.index('    /// `Slot.safeInsert`')]
    if not take.index('self.try_remove(')<take.index('self.on_take('):raise ValueError('Native output/consumption order changed')
    body=menu[menu.index('    fn on_take('):menu.index('    fn drop_stack(')]
    grid=body[body.index('let grid = self.containers[GRID].clone();'):]
    if grid.count('.find(&grid, n)')!=2 or not grid.index('.remainders(id, &grid, n)')<grid.index('stack.count -= 1;'):raise ValueError('Native menu pair route changed')
    crafter=(host/'crates/horizon-client/src/game/machines/crafter.rs').read_text()
    markers=['.find_with(&grid, 3,', 'Menu::map_crafted(', '.remainders(&recipe, &grid, 3)', 'self.crafter_eject(key, facing, result', 's.count -= 1;']
    positions=[crafter.index(value) for value in markers]
    if positions!=sorted(positions):raise ValueError('Native crafter pair route changed')
    return {path:common.digest((host/path).read_bytes()) for path in ('crates/horizon-client/src/menu.rs','crates/horizon-client/src/game/machines/crafter.rs')}
def verify(output):
    evidence=base.verify(output,expected_name=NAME)
    if routing(Path(evidence['official_source']))!=evidence['product_routing_sha256']:raise ValueError('Native product routing source drift')
    return evidence
def prepare(host,output):
    matching.prepare(host,output);native=ROOT/'mixins/pluto_native_creative'
    evidence=json.loads((output/'fixture-sources.json').read_text())
    for src,dst in [('crafting-products.rs','products.rs'),('nbt-storage.rs','nbt-storage.rs')]:shutil.copyfile(native/src,output/dst)
    with (output/'crafting.rs').open('a') as f:f.write('\n#[path="products.rs"] pub mod pluto_products;\n')
    shutil.copyfile(ROOT/'tests/native/crafting_products.rs',output/'lib.rs')
    policy=ROOT.parent/'tconstruct/tests/rust/crafting-product-policy'
    for src,dst in [('reference.rs','products-reference.rs'),('source_helpers.rs','products-source-helpers.rs'),('proposal.rs','products-proposal.rs')]:shutil.copyfile(policy/src,output/dst)
    get=lambda name:subprocess.check_output(['git','-C',str(ROOT.parent),'show',base.REFERENCE+':tconstruct/mixins/tconstruct/'+name],text=True)
    ref=get('source-overslime-crafting.rs');helpers=get('source-overslime.rs')
    expected=helpers[helpers.index('pub(super) fn refill('):helpers.index('/// Pinned source tag')]+helpers[helpers.index('pub(super) fn amount('):helpers.index('pub(super) fn upgrade(')]
    if (output/'products-reference.rs').read_text()!=ref or (output/'products-source-helpers.rs').read_text()!=expected:raise ValueError('Consumer reference drift')
    evidence['consumer_product_reference_sha256']=common.digest(ref.encode());evidence['consumer_product_helpers_sha256']=common.digest(expected.encode())
    (output/'Cargo.toml').write_text((output/'Cargo.toml').read_text().replace('name="pluto-crafting-match-tests"','name="'+NAME+'"'))
    evidence['files_sha256']={str(p.relative_to(output)):common.digest(p.read_bytes()) for p in output.rglob('*') if p.is_file() and p.name not in ('Cargo.lock','fixture-sources.json')}
    evidence['product_routing_sha256']=routing(host)
    evidence['scope']='Explicit pair staging, exact native Input and real ItemStack/Items/codecs. No assembly/remainder hooks or Menu consumption. Inherited matching wrappers/special endpoints are not additional executed coverage.'
    common.metadata(output,False);(output/'fixture-sources.json').write_text(json.dumps(evidence,indent=2)+'\n');verify(output);print(output)
if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--horizon-source',type=Path);p.add_argument('--output',type=Path,required=True);p.add_argument('--verify-lock',action='store_true');args=p.parse_args()
    if args.verify_lock:verify(args.output);print('Exact product source and dependency identities verified')
    elif args.horizon_source is None:p.error('--horizon-source required')
    else:prepare(args.horizon_source,args.output)
