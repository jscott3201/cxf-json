import hashlib
import importlib.util
import os
import tempfile
import unittest
from pathlib import Path


module_path = Path(__file__).with_name("check-fixture-checksums.py")
spec = importlib.util.spec_from_file_location("check_fixture_checksums", module_path)
if spec is None or spec.loader is None:
    raise RuntimeError("failed to load fixture checksum checker")
checksums = importlib.util.module_from_spec(spec)
spec.loader.exec_module(checksums)


class FixtureChecksumTests(unittest.TestCase):
    def setUp(self):
        self.temporary_directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary_directory.cleanup)
        self.root = Path(self.temporary_directory.name)
        self.fixture = self.root / "case.jsonld"
        self.fixture.write_bytes(b"{}\n")
        self.provenance = self.root / "PROVENANCE.md"

    def write_entry(self, digest, name="case.jsonld"):
        self.provenance.write_text(
            f"# Provenance\n\n- `{digest}` `{name}`\n"
        )

    def test_accepts_exact_inventory(self):
        self.write_entry(hashlib.sha256(self.fixture.read_bytes()).hexdigest())
        checksums.verify(self.provenance, self.root)

    def test_rejects_changed_bytes(self):
        self.write_entry(hashlib.sha256(self.fixture.read_bytes()).hexdigest())
        self.fixture.write_bytes(b"{ }\n")
        with self.assertRaisesRegex(ValueError, "checksum mismatch"):
            checksums.verify(self.provenance, self.root)

    def test_rejects_missing_and_extra_entries(self):
        digest = hashlib.sha256(self.fixture.read_bytes()).hexdigest()
        self.write_entry(digest, "missing.jsonld")
        with self.assertRaisesRegex(ValueError, "inventory mismatch"):
            checksums.verify(self.provenance, self.root)

    def test_rejects_path_escape_and_duplicate_entries(self):
        digest = hashlib.sha256(self.fixture.read_bytes()).hexdigest()
        self.write_entry(digest, "../case.jsonld")
        with self.assertRaisesRegex(ValueError, "must be a filename"):
            checksums.verify(self.provenance, self.root)

        self.provenance.write_text(
            f"- `{digest}` `case.jsonld`\n- `{digest}` `case.jsonld`\n"
        )
        with self.assertRaisesRegex(ValueError, "duplicate checksum"):
            checksums.verify(self.provenance, self.root)

    @unittest.skipUnless(hasattr(os, "symlink"), "symlinks are unavailable")
    def test_rejects_provenance_and_fixture_symlinks(self):
        digest = hashlib.sha256(self.fixture.read_bytes()).hexdigest()
        target = self.root / "target.md"
        target.write_text(f"- `{digest}` `case.jsonld`\n")
        provenance_link = self.root / "PROVENANCE-LINK.md"
        provenance_link.symlink_to(target)
        with self.assertRaisesRegex(ValueError, "without following symlinks"):
            checksums.verify(provenance_link, self.root)

        self.write_entry(digest)
        external = self.root / "external.data"
        external.write_bytes(self.fixture.read_bytes())
        self.fixture.unlink()
        self.fixture.symlink_to(external)
        with self.assertRaisesRegex(ValueError, "without following symlinks"):
            checksums.verify(self.provenance, self.root)

    @unittest.skipUnless(hasattr(os, "symlink"), "symlinks are unavailable")
    def test_rejects_symlinked_ancestor(self):
        real = self.root / "real"
        real.mkdir()
        fixture = real / "case.jsonld"
        fixture.write_bytes(b"{}\n")
        provenance = real / "PROVENANCE.md"
        digest = hashlib.sha256(fixture.read_bytes()).hexdigest()
        provenance.write_text(f"- `{digest}` `case.jsonld`\n")
        linked = self.root / "linked"
        linked.symlink_to(real, target_is_directory=True)
        with self.assertRaisesRegex(ValueError, "without following symlinks"):
            checksums.verify(linked / "PROVENANCE.md", self.root)

    def test_rejects_parent_traversal(self):
        escaped = self.root / "safe" / ".." / ".." / "PROVENANCE.md"
        with self.assertRaisesRegex(ValueError, "must not contain parent traversal"):
            checksums.verify(escaped, self.root)


if __name__ == "__main__":
    unittest.main()
