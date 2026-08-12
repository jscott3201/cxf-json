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

    def test_bounds_candidate_path_fanout(self):
        paths = {f"path-{index}" for index in range(history.MAX_CANDIDATE_PATHS + 2)}

        candidates, truncated = history.secret_candidates(
            b"gh" + b"p_" + b"A" * 36,
            "1" * 40,
            "blob",
            paths,
        )

        self.assertFalse(truncated)
        self.assertEqual(len(candidates[0]["paths"]), history.MAX_CANDIDATE_PATHS)
        self.assertEqual(candidates[0]["omitted_path_count"], 2)

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
        self.assertIn("\\[REDACTED:assigned-secret\\]", report)

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

    def test_report_is_deterministic(self):
        first = history.render_report(self.inventory(), "inventory command")
        second = history.render_report(self.inventory(), "inventory command")
        self.assertEqual(first, second)
        self.assertIn("Status: INCOMPLETE", first)

    def test_writes_report_atomically(self):
        report = Path(self.temporary_directory.name) / "report.md"

        history.write_report(report, "complete\n")

        self.assertEqual(report.read_text(), "complete\n")
        self.assertEqual(list(report.parent.glob(f".{report.name}.*.tmp")), [])

    def test_failed_report_replace_preserves_existing_report(self):
        report = Path(self.temporary_directory.name) / "report.md"
        report.write_text("prior\n")

        with (
            mock.patch.object(os, "replace", side_effect=OSError("replace failed")),
            self.assertRaises(OSError),
        ):
            history.write_report(report, "replacement\n")

        self.assertEqual(report.read_text(), "prior\n")
        self.assertEqual(list(report.parent.glob(f".{report.name}.*.tmp")), [])

    def test_excludes_local_only_ref_from_inventory_scope(self):
        self.write("private.txt", "not remotely advertised\n")
        self.commit("private history", "2026-08-03T00:00:00Z")
        private_commit = self.rev_parse("HEAD")
        mirror = Path(self.temporary_directory.name) / "local-only.git"
        self.git(
            "clone",
            "--mirror",
            str(self.repository),
            str(mirror),
            use_repository=False,
        )
        self.git("reset", "--hard", self.final_commit)
        subprocess.run(
            ["git", "-C", str(mirror), "update-ref", "refs/heads/main", self.final_commit],
            check=True,
        )
        subprocess.run(
            ["git", "-C", str(mirror), "update-ref", "refs/private/injected", private_commit],
            check=True,
        )
        subprocess.run(
            ["git", "-C", str(mirror), "remote", "set-url", "origin", str(self.repository)],
            check=True,
        )

        with self.assertRaises(history.InventoryError):
            history.collect_inventory(
                mirror,
                self.final_commit,
                str(self.repository),
                max_scanned_object_bytes=1_024,
                large_blob_bytes=48,
                allow_noncanonical_remote=True,
            )

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

    def test_ignores_transport_environment(self):
        with mock.patch.dict(
            os.environ,
            {"HTTPS_PROXY": "http://127.0.0.1:1", "SSL_CERT_FILE": "/missing"},
        ):
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

    def test_escapes_control_characters_in_report_cells(self):
        self.assertEqual(
            history.markdown("tab\treturn\rend"), "tab\\\\u0009return\\\\u000dend"
        )

    def test_escapes_markdown_links_and_images(self):
        rendered = history.markdown("![remote](https://example.test/pixel)")

        self.assertEqual(
            rendered, "\\!\\[remote\\]\\(https://example.test/pixel\\)"
        )

    def test_rejects_malformed_remote_advertisement(self):
        with mock.patch.object(history, "git_limited", return_value=b"malformed\n"):
            with self.assertRaises(history.InventoryError):
                history.collect_remote_refs(self.repository, str(self.repository))

    def test_rejects_unterminated_remote_advertisement(self):
        advertisement = f"{'1' * 40}\trefs/heads/main".encode()
        with (
            mock.patch.object(history, "git_limited", return_value=advertisement),
            self.assertRaises(history.InventoryError),
        ):
            history.collect_remote_refs(self.repository, str(self.repository))

    def test_rejects_incomplete_commit_enumeration(self):
        refs = [{"commit": self.final_commit}]
        with (
            mock.patch.object(history, "git_limited", return_value=self.final_commit.encode()),
            self.assertRaises(history.InventoryError),
        ):
            history.collect_commits(self.repository, refs)

    def test_rejects_malformed_tree_record(self):
        with (
            mock.patch.object(history, "git_limited", return_value=b"malformed\0"),
            self.assertRaises(history.InventoryError),
        ):
            history.tree_entries(self.repository, self.final_commit)

    def test_rejects_unterminated_tree_output(self):
        output = f"100644 blob {'1' * 40}\tpath".encode()
        with (
            mock.patch.object(history, "git_limited", return_value=output),
            self.assertRaises(history.InventoryError),
        ):
            history.tree_entries(self.repository, self.final_commit)

    def test_rejects_unterminated_local_ref_inventory(self):
        output = f"refs/heads/main\0commit\0{'1' * 40}\0".encode()
        with (
            mock.patch.object(history, "git_limited", return_value=output),
            self.assertRaises(history.InventoryError),
        ):
            history.collect_refs(self.repository)

    def test_rejects_git_output_above_limit(self):
        with self.assertRaises(history.InventoryError):
            history.git_limited(
                self.repository,
                1,
                "for-each-ref",
                "--format=%(refname)",
            )

    def test_rejects_non_posix_platform_before_spawn(self):
        with (
            mock.patch.object(history.os, "name", "nt"),
            mock.patch.object(history.subprocess, "Popen") as popen,
            self.assertRaises(history.InventoryError),
        ):
            history.git_limited(self.repository, 1, "status")

        popen.assert_not_called()

    def test_rejects_remote_ref_count_above_limit(self):
        advertisement = (
            f"{'1' * 40}\trefs/heads/one\n{'2' * 40}\trefs/heads/two\n"
        ).encode()
        with (
            mock.patch.object(history, "MAX_REMOTE_REFS", 1),
            mock.patch.object(history, "git_limited", return_value=advertisement),
            self.assertRaises(history.InventoryError),
        ):
            history.collect_remote_refs(self.repository, str(self.repository))

    def test_rejects_historical_path_above_limit(self):
        with (
            mock.patch.object(history, "MAX_PATH_BYTES", 1),
            self.assertRaises(history.InventoryError),
        ):
            history.tree_entries(self.repository, self.final_commit)

    def test_rejects_aggregate_tree_entries_above_limit(self):
        with (
            mock.patch.object(history, "MAX_HISTORICAL_TREE_ENTRIES", 1),
            self.assertRaises(history.InventoryError),
        ):
            self.inventory()

    def test_rejects_aggregate_scanned_bytes_above_limit(self):
        with (
            mock.patch.object(history, "MAX_TOTAL_SCANNED_OBJECT_BYTES", 1),
            self.assertRaises(history.InventoryError),
        ):
            self.inventory()

    def test_rejects_aggregate_secret_candidates_above_limit(self):
        with (
            mock.patch.object(history, "MAX_TOTAL_SECRET_CANDIDATES", 0),
            self.assertRaises(history.InventoryError),
        ):
            self.inventory()

    def test_sanitizes_git_error_for_logs(self):
        with (
            mock.patch.object(
                history,
                "git_limited",
                side_effect=history.InventoryError(
                    history.redact_secret_text(
                        history.safe_log_text("password=supersecret\tbad\rremote")
                    )
                ),
            ),
            self.assertRaises(history.InventoryError) as raised,
        ):
            history.git_text(self.repository, "status")

        message = str(raised.exception)
        self.assertNotIn("supersecret", message)
        self.assertIn("[REDACTED:assigned-secret]", message)
        self.assertNotIn("\n", message)
        self.assertNotIn("\t", message)
        self.assertNotIn("\r", message)

    def test_preserves_ref_byte_identity(self):
        self.assertEqual(history.encode_ref(b"refs/heads/\xff"), "refs/heads/%FF")
        self.assertEqual(history.encode_ref(b"refs/heads/%FF"), "refs/heads/%25FF")

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

        with self.assertRaises(history.InventoryError):
            history.collect_inventory(
                mirror,
                self.final_commit,
                str(self.repository),
                allow_noncanonical_remote=True,
            )


if __name__ == "__main__":
    unittest.main()
