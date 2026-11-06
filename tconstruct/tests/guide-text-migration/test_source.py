"""Scope/reversal guards, separate from actual Font execution."""
import hashlib
import re
import struct
import zlib
from pathlib import Path
import tomllib
import unittest

ROOT = Path(__file__).resolve().parents[3]
SOURCE = ROOT / 'tconstruct/mixins/tconstruct'

def digest(data): return hashlib.sha256(data).hexdigest()

MODULE = '''[[module]]
id = "materials-book-text"
file = "crates/horizon-client/src/game/books.rs"
name = "tconstruct_materials_book_text"
source = "source-materials-book-text.rs"

'''
OLD_CLICK = '''        let lines = game.gui.atlas.font.split(text, 182.0);
        let Some(line) = lines.get(row as usize) else { continue; };
        let line = line.resolve_legacy();
        let mut x = mx - origin;
        for (c, style) in line.chars() {
            x -= game.gui.atlas.font.advance(c, style);
            if x < 0.0 { link = style.click.clone(); break; }
        }
'''
NEW_CLICK = '''        let Some(lines) = book_text.page(&game.gui.atlas.font, base + offset, 182.0) else { continue; };
        link = lines.click(&game.gui.atlas.font, mx - origin, row as usize);
'''
class Scope(unittest.TestCase):
    def test_click_reverse_is_exact_baseline_geometry_and_action_policy(self):
        source = (SOURCE / 'source-materials-book-click.rs').read_text()
        self.assertEqual(source.count(NEW_CLICK), 1)
        source = source.replace('    let book_text = tconstruct_materials_book_text::Book::new(pages);\n', '')
        source = source.replace('    for offset in 0..if cover { 0 } else { 2 } {\n',
            '    for offset in 0..if cover { 0 } else { 2 } {\n        let Some(text) = pages.get(base + offset) else { continue; };\n')
        source = source.replace(NEW_CLICK, OLD_CLICK)
        self.assertEqual(digest(source.encode()), '0d3e040e11f1679519f0c6a0f15a1e145f667907a02227798d6a9dfdbf9c618e')
    def test_draw_reverse_is_exact_baseline_content_and_geometry(self):
        source = (SOURCE / 'source-materials-book-draw.rs').read_text()
        source = source.replace('    let book_text = tconstruct_materials_book_text::Book::new(pages);\n', '')
        source = source.replace('    let Some(lines) = book_text.page(&game.gui.atlas.font, base + offset, 182.0) else { continue; };\n    for (i, line) in lines.as_slice().iter().take(17).enumerate() {',
            '    for (i, line) in game.gui.atlas.font.split(text, 182.0).iter().take(17).enumerate() {')
        self.assertEqual(digest(source.encode()), '99147312fc3fcc9db002aefb11f047d4d1341ba6bc2d2b7edcd2d2cda020070a')
    def test_manifest_adds_only_private_support_module_no_hook_or_state(self):
        source = (SOURCE / 'horizon-mixin.toml').read_text()
        self.assertEqual(source.count(MODULE), 1)
        self.assertEqual(digest(source.replace(MODULE, '').encode()), '4ac1fc2101e37574c6dda5761ebb3d0cca13975da3ca517a4e847e2522ce3504')
        entries = [m for m in tomllib.loads(source)['module'] if m['id'] == 'materials-book-text']
        self.assertEqual(len(entries), 1)
        self.assertEqual(entries[0]['file'], 'crates/horizon-client/src/game/books.rs')
    def test_adapter_retains_full_document_and_native_fallback(self):
        source = (SOURCE / 'source-materials-book-text.rs').read_text()
        for token in ['from_native(self.pages, |_| true)', 'layout::split_raw(font, doc, index, width)',
                      'Lines::Legacy(font.split(text, width))', 'style.click.clone()', 'OnceCell<Option<Document>>']:
            self.assertIn(token, source)
        for token in ['Game<', 'send_packet(', 'book_follow_link(', 'static ', 'Mutex', 'unsafe']:
            self.assertNotIn(token, source)
    def test_raw_api_has_no_eager_or_post_failure_native_queries(self):
        source = (ROOT / 'pluto-lib/mixins/pluto_native_creative/guide-layout.rs').read_text()
        split = source.split('pub fn split_raw(', 1)[1]
        self.assertEqual(split.count('font.'), 1)
        self.assertIn('Ok(RawLines { lines: font.split(&native, width) })', split)
        for token in ['advances(', 'width_text(', 'resolve_legacy(', 'Err(']:
            self.assertNotIn(token, split.split('let native =', 1)[1])
        hit = source.split('pub fn hit_native(', 1)[1].split('/// Bounded admission', 1)[0]
        self.assertEqual(hit.count('font.'), 1)
        self.assertEqual(hit.count('resolve_legacy()'), 1)
        self.assertIn('return Some(NativeHit', hit)
        self.assertNotIn('Result<', hit)
    def test_original_bitmap_and_intended_full_region_are_explicit(self):
        source = (Path(__file__).parent / 'native.rs').read_text()
        values = re.search(r'const ORIGINAL_BITMAP: &\[u8\] = &\[([^]]+)\]', source).group(1)
        png = bytes(int(value) for value in values.split(','))
        self.assertEqual(png[:8], b'\x89PNG\r\n\x1a\n')
        offset, chunks = 8, {}
        while offset < len(png):
            size = int.from_bytes(png[offset:offset+4], 'big')
            kind = png[offset+4:offset+8]
            data = png[offset+8:offset+8+size]
            crc = int.from_bytes(png[offset+8+size:offset+12+size], 'big')
            self.assertEqual(crc, zlib.crc32(kind + data))
            chunks[kind] = data
            offset += size + 12
        self.assertEqual(chunks[b'IHDR'], struct.pack('!2I5B', 16, 8, 8, 6, 0, 0, 0))
        raw = zlib.decompress(chunks[b'IDAT'])
        row = b'\0' + b''.join(bytes([255, 255, 255, 255 if x < 2 or 8 <= x < 14 else 0]) for x in range(16))
        self.assertEqual(raw, row * 8)
        self.assertIn('before.len(), if full { 1 } else { 2 }', source)
        self.assertIn("old.1.glyph('b').rect.is_none(), full", source)
        self.assertIn("new.1.glyph('b').advance, if full { 6.0 } else { 7.0 }", source)

    def test_fixture_is_actual_native_font_and_preserves_fallback_cases(self):
        source = (Path(__file__).parent / 'native.rs').read_text()
        self.assertIn('Font::metrics(&root)', source)
        self.assertIn('font.split(text, width)', source)
        self.assertEqual(source.count('#[test]'), 12)
        for token in ['RunCommand', 'OpenUrl', 'CopyToClipboard', 'f32::NAN', 'usize::MAX', '8193', '4097', '1e100']:
            self.assertIn(token, source)

if __name__ == '__main__': unittest.main()
