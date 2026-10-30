"""Exercise portable extraction of pinned Git archive file links."""
import io
import subprocess
import threading
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

    def test_drains_trailing_pipe_data_before_waiting_for_producer(self):
        data = io.BytesIO()
        with tarfile.open(fileobj=data, mode='w') as archive:
            item = tarfile.TarInfo('file.txt')
            payload = b'original'
            item.size = len(payload)
            archive.addfile(item, io.BytesIO(payload))
        code = ("import sys; sys.stdout.buffer.write(bytes.fromhex(sys.argv[1])); "
                "sys.stdout.buffer.write(b'\\0' * (2 * 1024 * 1024)); sys.stdout.flush()")
        proc = subprocess.Popen([sys.executable, '-c', code, data.getvalue().hex()], stdout=subprocess.PIPE)
        watchdog = threading.Timer(5, proc.kill)
        watchdog.start()
        try:
            with tempfile.TemporaryDirectory() as tmp:
                extract_git_tar(proc.stdout, Path(tmp))
                self.assertEqual((Path(tmp) / 'file.txt').read_bytes(), b'original')
            self.assertEqual(proc.wait(timeout=5), 0)
        finally:
            watchdog.cancel()
            proc.stdout.close()
            if proc.poll() is None:
                proc.kill()
                proc.wait()

    def test_escape_link_is_rejected(self):
        data = io.BytesIO()
        with tarfile.open(fileobj=data, mode='w') as archive:
            link = tarfile.TarInfo('escape'); link.type = tarfile.SYMTYPE; link.linkname = '../outside'; archive.addfile(link)
        data.seek(0)
        with tempfile.TemporaryDirectory() as tmp, self.assertRaises((tarfile.FilterError, ValueError, OSError)):
            extract_git_tar(data, Path(tmp))
