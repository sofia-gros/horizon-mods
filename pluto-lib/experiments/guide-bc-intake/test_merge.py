"""Pure pinned-source merge checks, not Rust or installed behavior tests."""
import importlib.util,os,tomllib,unittest
from pathlib import Path
HERE=Path(__file__).resolve().parent
s=importlib.util.spec_from_file_location('guide_merge',HERE/'merge.py');m=importlib.util.module_from_spec(s);s.loader.exec_module(m)
NAV=Path(os.environ['GUIDE_NAVIGATION_INPUT']);BC=Path(os.environ['GUIDE_CONSUMER_INPUT'])
def read(root):return {name:(root/name).read_bytes() for name in m.NAMES}
class Checks(unittest.TestCase):
    def test_two_complete_reverse_proofs(self):
        # compose executes both whole-file inverse assertions for both inputs.
        result=m.compose(read(NAV),read(BC));self.assertEqual(set(result),set(m.NAMES))
    def test_all_navigation_injections_remain_exact(self):
        nav=tomllib.loads(read(NAV)[m.NAMES[0]].decode());out=tomllib.loads(m.compose(read(NAV),read(BC))[m.NAMES[0]].decode())
        self.assertEqual(nav['injection'],out['injection'])
        self.assertTrue(any(x['id']=='materials-book-control' for x in out['injection']))
        self.assertFalse(any(x['id']=='materials-book-turn' for x in out['injection']))
    def test_only_two_exact_consumer_modules_added(self):
        nav=tomllib.loads(read(NAV)[m.NAMES[0]].decode());bc=tomllib.loads(read(BC)[m.NAMES[0]].decode());out=tomllib.loads(m.compose(read(NAV),read(BC))[m.NAMES[0]].decode())
        extra=[x for x in out['module'] if x not in nav['module']]
        self.assertEqual({x['id'] for x in extra},{'materials-book-text','materials-book-identity'})
        self.assertEqual(extra,[x for x in bc['module'] if x['id'] in {'materials-book-text','materials-book-identity'}])
        self.assertEqual([x for x in out['module'] if x not in extra],nav['module'])
    def test_click_admission_links_navigation_history_tail_unchanged(self):
        nav=read(NAV)[m.NAMES[1]];out=m.compose(read(NAV),read(BC))[m.NAMES[1]]
        prefix=b'    let mut link = None;\n';tail=b'    if let Some(link) = link {\n'
        self.assertEqual(nav.split(prefix)[0],out.split(prefix)[0]);self.assertEqual(nav.split(tail)[1],out.split(tail)[1])
        self.assertEqual(out.count(b'tconstruct_materials_book_navigation::turn(game,'),2)
        self.assertNotIn(b'game.turn_page(',out)
    def test_query_is_the_exact_reviewed_lazy_loop(self):
        bc=read(BC)[m.NAMES[1]];out=m.compose(read(NAV),read(BC))[m.NAMES[1]]
        a=b'    let mut link = None;\n';b=b'    if let Some(link) = link {\n'
        self.assertEqual(m.section(out,a,b),m.section(bc,a,b))
        self.assertNotIn(b'font.advance(',m.section(out,a,b))
    def test_source_mutation_missing_or_extra_rejected(self):
        for group in ['nav','bc']:
            for name in m.NAMES:
                nav=read(NAV);bc=read(BC);chosen=nav if group=='nav' else bc;chosen[name]+=b'\n'
                with self.assertRaises(ValueError):m.compose(nav,bc)
        nav=read(NAV);nav['extra']=b''
        with self.assertRaises(ValueError):m.compose(nav,read(BC))
    def test_unique_boundary_rejects_ambiguous_or_absent(self):
        for raw in [b'',b'xx']:
            with self.assertRaises(ValueError):m.replace(raw,b'x',b'y')
        with self.assertRaises(ValueError):m.section(b'aabab',b'a',b'b')
    def test_other_manifest_tables_unchanged(self):
        nav=tomllib.loads(read(NAV)[m.NAMES[0]].decode());out=tomllib.loads(m.compose(read(NAV),read(BC))[m.NAMES[0]].decode())
        self.assertEqual({k:v for k,v in nav.items() if k!='module'},{k:v for k,v in out.items() if k!='module'})
if __name__=='__main__':unittest.main()
