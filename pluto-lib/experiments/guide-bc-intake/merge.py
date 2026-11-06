"""Only two source merges. No Git, native code, metadata, network or stage changes."""
from pathlib import Path
import hashlib,json
HERE=Path(__file__).resolve().parent
NAMES=('horizon-mixin.toml','source-materials-book-click.rs')
def sha(raw):return hashlib.sha256(raw).hexdigest()
def require(ok,reason):
    if not ok:raise ValueError(reason)
def replace(raw,old,new):
    require(raw.count(old)==1 and old!=new,'unique merge boundary')
    return raw.replace(old,new,1)
def section(raw,start,end):
    require(raw.count(start)==raw.count(end)==1,'unique section bounds')
    a=raw.index(start);b=raw.index(end,a);return raw[a:b]
def compose(navigation,consumer):
    cfg=json.loads((HERE/'inputs.json').read_text())
    for group,files in [('navigation',navigation),('consumer',consumer)]:
        require(set(files)==set(NAMES),'exact source set')
        for name in NAMES:require(sha(files[name])==cfg[group][name],'source drift: '+group+'/'+name)
    before=b'    let mut link = None;\n';after=b'    if let Some(link) = link {\n'
    nav_query=section(navigation[NAMES[1]],before,after)
    consumer_query=section(consumer[NAMES[1]],before,after)
    click=replace(navigation[NAMES[1]],nav_query,consumer_query)
    require(replace(click,consumer_query,nav_query)==navigation[NAMES[1]],'navigation click inverse')
    without_nav=click
    for direction in [b'true',b'false']:
        without_nav=replace(without_nav,b'tconstruct_materials_book_navigation::turn(game, '+direction+b')',b'game.turn_page('+direction+b')')
    require(without_nav==consumer[NAMES[1]],'consumer click inverse')
    manifest=navigation[NAMES[0]]
    added=[]
    for name,next_name in [(b'text',b'index'),(b'identity',b'wire')]:
        start=b'[[module]]\nid = "materials-book-'+name+b'"\n'
        end=b'[[module]]\nid = "materials-book-'+next_name+b'"\n'
        chunk=section(consumer[NAMES[0]],start,end)
        require(start not in manifest,'module already installed')
        manifest=replace(manifest,end,chunk+end);added.append(chunk)
    without_modules=manifest
    for chunk in added:without_modules=replace(without_modules,chunk,b'')
    require(without_modules==navigation[NAMES[0]],'navigation manifest inverse')
    start=b'# Spread navigation is consumer policy, not a replacement for native turn_page.\n'
    end=b'[[module]]\nid = "materials-book-spread"\n'
    nav_body=section(manifest,start,end)
    old_turn=section(consumer[NAMES[0]],b'[[injection]]\nid = "materials-book-turn"\n',end)
    require(replace(manifest,nav_body,old_turn)==consumer[NAMES[0]],'consumer manifest inverse')
    return {NAMES[0]:manifest,NAMES[1]:click}
