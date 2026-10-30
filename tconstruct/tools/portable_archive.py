"""Extract a Git tar stream, materializing safe file symlinks portably."""
import tarfile
from pathlib import Path


def extract_git_tar(stream, destination):
    root = Path(destination).resolve(strict=True)
    symlinks = []
    def safe_filter(member, path):
        member = tarfile.data_filter(member, path)
        if member.issym():
            symlinks.append(member)
            return None
        return member
    with tarfile.open(fileobj=stream, mode='r|') as archive:
        archive.extractall(root, filter=safe_filter)
    # tar ends before the producer necessarily finishes writing padding.
    # Consume the remaining pipe so git archive can exit before its caller waits.
    while stream.read(65536):
        pass
    for member in symlinks:
        dest = root / member.name
        source = (dest.parent / member.linkname).resolve(strict=True)
        if not source.is_relative_to(root) or not source.is_file():
            raise ValueError('Git archive symlink must resolve to an in-tree file')
        if dest.exists() or dest.is_symlink():
            raise ValueError('Git archive symlink destination already exists')
        dest.parent.mkdir(parents=True, exist_ok=True)
        dest.write_bytes(source.read_bytes())
