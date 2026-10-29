"""Exercise portable extraction of pinned Git archive file links."""
import io
from pathlib import Path
import sys
import tarfile
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'tools'))
from portable_archive import extract_git_tar

class PortableArchive(unittest.TestCase):
    def test_link_before_target_is_materialized(self):
        data = io.BytesIO()
        with tarfile.open(fileobj=data, mode='w') as archive:
            link = tarfile.TarInfo('CLAUDE.md'); link.type = tarfile.SYMTYPE; link.linkname = 'AGENTS.md'; archive.addfile(link)
            item = tarfile.TarInfo('AGENTS.md'); payload = b'original'; item.size = len(payload); archive.addfile(item, io.BytesIO(payload))
        data.seek(0)
        with tempfile.TemporaryDirectory() as tmp:
            extract_git_tar(data, Path(tmp))
            self.assertEqual((Path(tmp) / 'CLAUDE.md').read_bytes(), b'original')
            self.assertFalse((Path(tmp) / 'CLAUDE.md').is_symlink())

    def test_escape_link_is_rejected(self):
        data = io.BytesIO()
        with tarfile.open(fileobj=data, mode='w') as archive:
            link = tarfile.TarInfo('escape'); link.type = tarfile.SYMTYPE; link.linkname = '../outside'; archive.addfile(link)
        data.seek(0)
        with tempfile.TemporaryDirectory() as tmp, self.assertRaises((tarfile.FilterError, ValueError, OSError)):
            extract_git_tar(data, Path(tmp))
