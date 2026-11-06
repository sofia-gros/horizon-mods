#!/usr/bin/env python3
"""Read-only pinned-source inventory. This does not run the native language loader."""
import argparse
import hashlib
import io
import json
from pathlib import Path
import re
import subprocess
import zipfile

HERE=Path(__file__).resolve()
META=HERE.parents[2]/'docs/fluid-language-audit-20261106.json'
def sha(data):return hashlib.sha256(data).hexdigest()
def inspect(repo,official):
    meta=json.loads(META.read_text())
    def read(path):return subprocess.check_output(['git','-C',str(repo),'show',meta['mod_revision']+':'+path])
    for path,digest in meta['mod_files'].items():assert sha(read(path))==digest,path
    for path,digest in meta['official_files'].items():assert sha((official/path).read_bytes())==digest,path
    ids=json.loads(read('tconstruct/content/reference/tconstruct-melter-fluids.json'))
    source=read('tconstruct/mixins/tconstruct/source-fluid-display.rs').decode().split('pub(super) fn sprite')[0]
    aliases=dict(re.findall(r'"([^"\n]+)" => "([^"\n]+)",',source))
    names=sorted({aliases.get(x,x) for x in ids})
    pairs={aliases[x] for x in ids if x in aliases}
    assert len(ids)==142 and len(names)==72 and len(pairs)==70
    assert sorted(set(names)-pairs)==['tconstruct:potion','tconstruct:powdered_snow']
    english=json.loads(read('tconstruct/content/assets/tconstruct/lang/en_us.json'))
    with zipfile.ZipFile(io.BytesIO(read('tconstruct/content/reference/tconstruct-part-visuals.zip'))) as archive:
        japanese=json.loads(archive.read('assets/tconstruct/lang/ja_jp.json'))
    for language in [english,japanese]:
        assert all('fluid.'+x.replace(':','.') in language for x in names)
        missing=[x for x in names if 'item.'+x.replace(':','.')+'_bucket' not in language]
        assert missing==['tconstruct:powdered_snow']
    assert japanese['fluid.tconstruct.molten_iron']=='溶融した鉄'
    with zipfile.ZipFile(io.BytesIO(read('tconstruct/content/reference/tconstruct-tool-visuals.zip'))) as archive:
        assert not any(p.startswith('assets/tconstruct/lang/') for p in archive.namelist())
    tracked=subprocess.check_output(['git','-C',str(repo),'ls-tree','-r','--name-only',meta['mod_revision'],'tconstruct/content/assets'],text=True).splitlines()
    assert 'tconstruct/content/assets/tconstruct/lang/ja_jp.json' not in tracked
    names_source=(official/'crates/horizon-client/src/creative.rs').read_text()
    body=names_source.split('pub fn display_get(&self, key: &str) -> Option<String> {',1)[1].split('pub fn get(',1)[0]
    assert body.index('crate::lang::current()')<body.index('self.get(key)')
    generated=read('pluto-lib/mixins/pluto_native_creative/api/src/fluid.rs').decode().split('pub fn content(def:',1)[1].split('#[cfg(test)]',1)[0]
    assert '/lang/' not in generated and 'horizon/item/' not in generated
    return {'source_inventory':'passed','reference_ids':142,'display_families':72,'flow_still_pairs':70,'en_us_fluid_keys':72,'ja_jp_fluid_keys':72,'bucket_keys_each':71,'native_loader_or_display_executed':False}
if __name__=='__main__':
    parser=argparse.ArgumentParser();parser.add_argument('--repo',type=Path,required=True);parser.add_argument('--official',type=Path,required=True);args=parser.parse_args()
    print(json.dumps(inspect(args.repo,args.official),sort_keys=True))
