#!/usr/bin/env python3
"""Test production route with real host state/NBT; world and structure are mocks."""
import os, pathlib, subprocess, tempfile, tomllib, shutil
root=pathlib.Path(__file__).resolve().parents[1]; host=pathlib.Path(os.environ['HORIZON_HOST']).resolve()
expected = '0c7b5b61a5e746d621afffba919b12aeccf9c0fa'
actual = subprocess.check_output(['git', '-C', str(host), 'rev-parse', 'HEAD'], text=True).strip()
dirty = subprocess.check_output(['git', '-C', str(host), 'status', '--porcelain', '--untracked-files=no'], text=True).strip()
if actual != expected or dirty:
 raise SystemExit(f'HORIZON_HOST must have a clean tracked tree at {expected}')
s=(root/'mixins/tconstruct/source-machine-runtime.rs').read_text()
helpers=s[s.index('fn selected_smeltery_fluid('):s.index('fn matched_casting(')]
route=s[s.index('fn patch_smeltery_controller_tank('):s.index('fn tank_casting_recipe_current(')] + s[s.index('fn faucet_network_tag_safe('):s.index('fn tank_to_casting_tank(')]

# Exercise the actual dispatch prefixes before the first transfer branch.
# Reaching any downstream transfer is a test failure, not a mock success.
def guarded_prefix(start, end):
    body = s[s.index(start):]
    body = body[:body.index(end)]
    return body + 'panic!("unsupported Duct reached a transfer route");\n}\n'
dispatch = guarded_prefix('pub(super) fn activate_faucet(', '    if let Some(handled) = tank_to_cannon(')
dispatch += guarded_prefix('pub(super) fn tick_faucet(', '    // Wait if either chunk')
template=(root/'tests/rust/faucet_smeltery_tank_fixture.rs').read_text().replace('__MIXINS__',str(root/'mixins/tconstruct')).replace('__PLUTO__',str(root.parent/'pluto-lib/mixins/pluto_native_creative/stack-fluid.rs')).replace('// __SMELTERY_HELPERS__',helpers).replace('// __PRODUCTION_ROUTE__',route).replace('// __DUCT_DISPATCH__',dispatch)
with tempfile.TemporaryDirectory(prefix='smeltery-faucet-') as tmp:
 p=pathlib.Path(tmp);(p/'src').mkdir();(p/'src/lib.rs').write_text(template)
 (p/'Cargo.toml').write_text('[package]\nname="smeltery-faucet-fixture"\nversion="0.1.0"\nedition="2024"\n[dependencies]\nhorizon-items={path="%s"}\nhorizon-world={path="%s"}\nhorizon-client-core={path="%s"}\n' % (host/'crates/horizon-items',host/'crates/horizon-world',host/'crates/horizon-client-core'))
 # Seed the fixture from the verified host lock, then permit Cargo to prune
 # irrelevant workspace packages. Reject any changed external dependency.
 shutil.copyfile(host/'Cargo.lock', p/'Cargo.lock')
 with (p/'Cargo.lock').open('a') as lock:
  lock.write('\n[[package]]\nname="smeltery-faucet-fixture"\nversion="0.1.0"\ndependencies=["horizon-items","horizon-world","horizon-client-core"]\n')
 subprocess.run(['cargo','metadata','--offline','--manifest-path',str(p/'Cargo.toml'),'--format-version','1'],check=True,stdout=subprocess.DEVNULL)
 def identity(package):
  return tuple(package.get(key) for key in ('name','version','source','checksum'))
 allowed = {identity(package) for package in tomllib.loads((host/'Cargo.lock').read_text())['package']}
 unknown = [identity(package) for package in tomllib.loads((p/'Cargo.lock').read_text())['package']
            if package['name'] != 'smeltery-faucet-fixture' and identity(package) not in allowed]
 if unknown:
  raise SystemExit(f'fixture dependencies differ from the pinned host lock: {unknown}')
 result = subprocess.run(['cargo','test','--locked','--offline','--manifest-path',str(p/'Cargo.toml')],
                         check=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True)
 print(result.stdout, end='')
 for name in ('runtime::duct_dispatch_preserves_all_fluid_and_unknown_metadata',
              'tconstruct_source_machine_interaction::tests::both_ducts_are_closed_without_affecting_drains_or_other_blocks',
              'runtime::reserves_atomically_and_preserves_opaque_controller'):
  if f'test {name} ... ok' not in result.stdout.splitlines():
   raise SystemExit(f'required production dispatch/control test did not pass: {name}')
