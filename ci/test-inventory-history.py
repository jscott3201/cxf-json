import hashlib
import importlib.util
import os
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest import mock


module_path = Path(__file__).with_name("inventory-history.py")
spec = importlib.util.spec_from_file_location("inventory_history", module_path)
if spec is None or spec.loader is None:
    raise RuntimeError("failed to load history inventory")
history = importlib.util.module_from_spec(spec)
spec.loader.exec_module(history)


class HistoryInventoryTests(unittest.TestCase):
    def setUp(self):
        self.temporary_directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary_directory.cleanup)
        self.repository = Path(self.temporary_directory.name) / "repository"
        self.git("init", "-b", "main", str(self.repository), use_repository=False)
        self.git("config", "user.name", "History Test")
        self.git("config", "user.email", "history@example.test")
        self.git("remote", "add", "origin", str(self.repository))

        secret = "gh" + "p_" + "A" * 36
        self.write("deleted.txt", f"token={secret}\n")
        self.write("LICENSE-MIT", "test MIT license\n")
        self.write("LICENSE-APACHE", "test Apache license\n")
        self.commit("initial", "2026-08-01T00:00:00Z")
        self.initial_commit = self.rev_parse("HEAD")
        self.git("tag", "lightweight")
        self.git("tag", "-a", "annotated", "-m", "release tag")
        self.git("update-ref", "refs/pull/1/head", self.initial_commit)

        (self.repository / "deleted.txt").unlink()
        self.write("src.txt", "current\n")
        self.write("artifact.wasm", b"\0asm")
        self.write("large.dat", b"x" * 64)
        self.commit("current", "2026-08-02T00:00:00Z")
        self.final_commit = self.rev_parse("HEAD")

    def git(self, *arguments, use_repository=True, check=True):
        command = ["git"]
        if use_repository:
            command.extend(["-C", str(self.repository)])
        command.extend(arguments)
        environment = os.environ.copy()
        environment.update(
            {
                "GIT_AUTHOR_DATE": "2026-08-01T00:00:00Z",
                "GIT_COMMITTER_DATE": "2026-08-01T00:00:00Z",
            }
        )
        return subprocess.run(
            command,
            check=check,
            capture_output=True,
            text=True,
            env=environment,
        )

    def write(self, relative_path, contents):
        path = self.repository / relative_path
        path.parent.mkdir(parents=True, exist_ok=True)
        if isinstance(contents, bytes):
            path.write_bytes(contents)
        else:
            path.write_text(contents)

    def commit(self, message, date):
        self.git("add", "-A")
        environment = os.environ.copy()
        environment.update({"GIT_AUTHOR_DATE": date, "GIT_COMMITTER_DATE": date})
        subprocess.run(
            ["git", "-C", str(self.repository), "commit", "-m", message],
            check=True,
            capture_output=True,
            text=True,
            env=environment,
        )

    def rev_parse(self, revision):
        return self.git("rev-parse", revision).stdout.strip()

    def inventory(self):
        return history.collect_inventory(
            self.repository,
            self.final_commit,
            str(self.repository),
            max_scanned_object_bytes=1_024,
            large_blob_bytes=48,
            allow_noncanonical_remote=True,
        )

    def test_inventories_refs_deleted_blobs_and_author_metadata(self):
        inventory = self.inventory()

        self.assertEqual(len(inventory["refs"]), 4)
        self.assertEqual(len(inventory["commits"]), 2)
        self.assertIn(
            "deleted.txt",
            {path for blob in inventory["blobs"] for path in blob["paths"]},
        )
        self.assertEqual(inventory["commits"][0]["author_email"], "history@example.test")
        self.assertEqual(inventory["coverage_errors"], [])
        annotated = next(
            ref for ref in inventory["refs"] if ref["name"] == "refs/tags/annotated"
        )
        self.assertEqual(annotated["tag_metadata"]["tagger_email"], "history@example.test")

    def test_redacts_secret_candidates(self):
        inventory = self.inventory()
        report = history.render_report(inventory, "inventory command")

        self.assertEqual(
            [candidate["pattern"] for candidate in inventory["secret_candidates"]],
            ["github-token"],
        )
        self.assertIn("deleted.txt", report)
        self.assertNotIn("A" * 36, report)

    def test_redacts_secrets_in_commit_and_tag_subjects(self):
        secret = "password=" + "supersecret"
        self.git("commit", "--allow-empty", "-m", secret)
        self.final_commit = self.rev_parse("HEAD")
        self.git("tag", "-a", "sensitive", "-m", secret)

        inventory = self.inventory()
        report = history.render_report(inventory, "inventory command")

        self.assertEqual(
            {candidate["object_kind"] for candidate in inventory["secret_candidates"]},
            {"blob", "commit", "tag"},
        )
        self.assertNotIn(secret, report)
        self.assertIn("[REDACTED:assigned-secret]", report)

    def test_classifies_binary_large_generated_and_unscanned_blobs(self):
        inventory = history.collect_inventory(
            self.repository,
            self.final_commit,
            str(self.repository),
            max_scanned_object_bytes=32,
            large_blob_bytes=48,
            allow_noncanonical_remote=True,
        )
        blobs = {
            path: blob for blob in inventory["blobs"] for path in blob["paths"]
        }

        self.assertTrue(blobs["artifact.wasm"]["binary"])
        self.assertTrue(blobs["artifact.wasm"]["generated_candidate"])
        self.assertTrue(blobs["large.dat"]["large"])
        self.assertFalse(blobs["large.dat"]["scanned"])
        self.assertEqual(history.binary_label([blobs["large.dat"]]), "not scanned")
        self.assertFalse(inventory["machine_scan_complete"])
        self.assertTrue(
            any(
                "metadata exceeds the content scan cap" in error
                for error in inventory["coverage_errors"]
            )
        )

    def test_report_is_deterministic_and_changes_with_ref_set(self):
        first = history.render_report(self.inventory(), "inventory command")
        second = history.render_report(self.inventory(), "inventory command")
        self.assertEqual(first, second)
        self.assertIn("Status: INCOMPLETE", first)

        self.git("update-ref", "refs/pull/2/head", self.final_commit)
        changed = history.render_report(self.inventory(), "inventory command")
        self.assertNotEqual(first, changed)

    def test_rejects_unreachable_final_commit(self):
        with self.assertRaises(history.InventoryError):
            history.collect_inventory(
                self.repository,
                "0" * 40,
                str(self.repository),
                allow_noncanonical_remote=True,
            )

    def test_rejects_credential_bearing_remote_url(self):
        with self.assertRaises(history.InventoryError):
            history.collect_inventory(
                self.repository,
                self.final_commit,
                "https://token@example.test/repository.git",
            )

    def test_rejects_configured_remote_name(self):
        with self.assertRaises(history.InventoryError):
            history.collect_inventory(self.repository, self.final_commit, "origin")

    def test_rejects_shallow_repository(self):
        shallow = Path(self.temporary_directory.name) / "shallow"
        self.git(
            "clone",
            "--depth",
            "1",
            f"file://{self.repository}",
            str(shallow),
            use_repository=False,
        )

        with self.assertRaises(history.InventoryError):
            history.collect_inventory(
                shallow,
                self.final_commit,
                str(self.repository),
                allow_noncanonical_remote=True,
            )

    def test_rejects_repository_local_url_rewrites(self):
        self.git(
            "config",
            "url.https://example.test/.insteadOf",
            "https://github.com/",
        )

        with self.assertRaises(history.InventoryError):
            self.inventory()

    def test_ignores_inherited_git_dir(self):
        other = Path(self.temporary_directory.name) / "other"
        self.git("init", "-b", "main", str(other), use_repository=False)

        with mock.patch.dict(os.environ, {"GIT_DIR": str(other / ".git")}):
            inventory = self.inventory()

        self.assertEqual(inventory["final_commit"], self.final_commit)

    def test_reports_unsupported_nested_tag(self):
        self.git("tag", "-a", "inner", "-m", "inner")
        self.git("tag", "-a", "outer", "-m", "outer", "inner")

        inventory = self.inventory()

        self.assertIn(
            "refs/tags/outer uses an unsupported nested annotated tag",
            inventory["coverage_errors"],
        )

    def test_reports_non_commit_ref_without_calling_it_missing(self):
        blob = self.git("hash-object", "src.txt").stdout.strip()
        self.git("update-ref", "refs/tags/blob", blob)

        inventory = self.inventory()

        self.assertIn(
            "refs/tags/blob does not resolve to a commit",
            inventory["coverage_errors"],
        )
        self.assertNotIn(
            "remote ref is missing locally: refs/tags/blob",
            inventory["coverage_errors"],
        )

    def test_preserves_non_utf8_path_identity(self):
        self.assertEqual(history.encode_path(b"\xff"), "%FF")
        self.assertEqual(history.encode_path(b"%FF"), "%25FF")

    def test_redacts_secret_bearing_path_and_retains_identity_digest(self):
        secret = b"gh" + b"p_" + b"A" * 36

        encoded = history.encode_path(secret)

        self.assertNotIn("A" * 36, encoded)
        self.assertIn(hashlib.sha256(secret).hexdigest(), encoded)

    def test_reports_remote_ref_missing_from_local_inventory(self):
        mirror = Path(self.temporary_directory.name) / "mirror.git"
        self.git(
            "clone",
            "--mirror",
            str(self.repository),
            str(mirror),
            use_repository=False,
        )
        self.git("update-ref", "refs/pull/2/head", self.final_commit)

        inventory = history.collect_inventory(
            mirror,
            self.final_commit,
            str(self.repository),
            allow_noncanonical_remote=True,
        )

        self.assertEqual(
            inventory["coverage_errors"],
            ["remote ref is missing locally: refs/pull/2/head"],
        )


if __name__ == "__main__":
    unittest.main()
