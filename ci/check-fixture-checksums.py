import hashlib
import os
import re
import stat
import sys
from pathlib import Path
from typing import NoReturn


ENTRY = re.compile(r"^- `([0-9a-f]{64})` `([^`]+)`$")


def fail(message) -> NoReturn:
    raise ValueError(message)


def open_fixture_directory(path, trusted_root):
    if ".." in path.parts:
        fail(f"path must not contain parent traversal: {path}")
    path = path.absolute()
    trusted_root = trusted_root.absolute()
    try:
        relative = path.relative_to(trusted_root)
    except ValueError:
        fail(f"path escapes trusted root: {path}")
    if not relative.parts:
        fail("provenance path must name a file")

    directory_flags = os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW
    directory = None
    try:
        directory = os.open(trusted_root, directory_flags)
        for part in relative.parts[:-1]:
            next_directory = os.open(part, directory_flags, dir_fd=directory)
            os.close(directory)
            directory = next_directory
    except OSError as error:
        if directory is not None:
            os.close(directory)
        fail(f"cannot open fixture directory without following symlinks: {error}")
    return directory, relative.parts[-1]


def read_regular_file(directory, name):
    if Path(name).name != name:
        fail(f"checksum path must be a filename: {name}")
    try:
        descriptor = os.open(name, os.O_RDONLY | os.O_NOFOLLOW, dir_fd=directory)
    except OSError as error:
        fail(f"cannot open regular file without following symlinks: {name}: {error}")
    try:
        if not stat.S_ISREG(os.fstat(descriptor).st_mode):
            fail(f"fixture is not a regular file: {name}")
        with os.fdopen(descriptor, "rb", closefd=False) as file:
            return file.read()
    finally:
        os.close(descriptor)


def verify(provenance, trusted_root=None):
    trusted_root = Path.cwd() if trusted_root is None else trusted_root
    directory, provenance_name = open_fixture_directory(provenance, trusted_root)
    try:
        provenance_bytes = read_regular_file(directory, provenance_name)
        entries = {}
        for line in provenance_bytes.decode().splitlines():
            if not line.startswith("- `"):
                continue
            match = ENTRY.fullmatch(line)
            if match is None:
                fail(f"malformed checksum entry: {line}")
            digest, name = match.groups()
            if Path(name).name != name:
                fail(f"checksum path must be a filename: {name}")
            if name in entries:
                fail(f"duplicate checksum entry: {name}")
            entries[name] = digest

        fixtures = {name for name in os.listdir(directory) if name.endswith(".jsonld")}
        if set(entries) != fixtures:
            missing = sorted(fixtures - set(entries))
            extra = sorted(set(entries) - fixtures)
            fail(f"checksum inventory mismatch: missing={missing}, extra={extra}")

        for name, expected in entries.items():
            actual = hashlib.sha256(read_regular_file(directory, name)).hexdigest()
            if actual != expected:
                fail(f"checksum mismatch for {name}: expected {expected}, found {actual}")
    finally:
        os.close(directory)


def main():
    if len(sys.argv) != 2:
        raise SystemExit("usage: check-fixture-checksums.py PROVENANCE.md")
    try:
        verify(Path(sys.argv[1]))
    except (OSError, ValueError) as error:
        raise SystemExit(f"fixture checksum violation: {error}") from error


if __name__ == "__main__":
    main()
