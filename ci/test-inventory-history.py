import hashlib
import errno
import importlib.util
import os
import shutil
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
        git_executable = shutil.which("git")
        if git_executable is None:
            self.fail("git executable was not found")
        self.git_executable = str(Path(git_executable).resolve())
        self.git_executable_sha256 = history.program_sha256(Path(self.git_executable))
        environment = history.git_environment()
        exec_path = history.run_limited_process(
            [self.git_executable, "--exec-path"], environment, history.MAX_PATH_BYTES
        ).decode().strip()
        self.git_https_helper_sha256 = history.program_sha256(
            Path(exec_path, "git-remote-https").resolve()
        )
        self.runner = history.GitRunner(
            self.git_executable,
            self.git_executable_sha256,
            self.git_https_helper_sha256,
        )
        self.addCleanup(self.runner.close)
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
            git_executable=self.git_executable,
            git_executable_sha256=self.git_executable_sha256,
            git_https_helper_sha256=self.git_https_helper_sha256,
        )

    def test_inventories_refs_deleted_blobs_and_author_metadata(self):
        inventory = self.inventory()

        self.assertEqual(len(inventory["refs"]), 4)
        self.assertEqual(len(inventory["commits"]), 2)
        self.assertIn(
            "deleted.txt",
            {
                path["display"]
                for blob in inventory["blobs"]
                for path in blob["path_records"]
            },
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

    def test_redacts_secret_with_control_whitespace_before_escaping(self):
        secret = "supersecret"
        self.git("commit", "--allow-empty", "-m", f"password=\t{secret}")
        self.final_commit = self.rev_parse("HEAD")

        report = history.render_report(self.inventory(), "inventory command")

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
            git_executable=self.git_executable,
            git_executable_sha256=self.git_executable_sha256,
            git_https_helper_sha256=self.git_https_helper_sha256,
        )
        blobs = {
            path["display"]: blob
            for blob in inventory["blobs"]
            for path in blob["path_records"]
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

    def test_report_lists_every_blob_object_id(self):
        inventory = self.inventory()
        report = history.render_report(inventory, "inventory command")

        for blob in inventory["blobs"]:
            self.assertIn(blob["object_id"], report)

    def test_writes_report_atomically(self):
        report = Path(self.temporary_directory.name) / "report.md"

        history.write_report(report, "complete\n")

        self.assertEqual(report.read_text(), "complete\n")
        self.assertEqual(list(report.parent.glob(f".{report.name}.*.tmp")), [])

    def test_fsyncs_report_directory_after_replace(self):
        report = Path(self.temporary_directory.name) / "report.md"
        real_fsync = os.fsync
        synced_directory = False

        def record_fsync(file_descriptor):
            nonlocal synced_directory
            descriptor_stat = os.fstat(file_descriptor)
            if (
                descriptor_stat.st_dev == os.stat(report.parent).st_dev
                and descriptor_stat.st_ino == os.stat(report.parent).st_ino
            ):
                synced_directory = True
            return real_fsync(file_descriptor)

        with mock.patch.object(os, "fsync", side_effect=record_fsync):
            history.write_report(report, "complete\n")

        self.assertTrue(synced_directory)

    def test_rejects_unsupported_report_directory_fsync(self):
        report = Path(self.temporary_directory.name) / "report.md"
        real_fsync = os.fsync

        def fail_directory_fsync(file_descriptor):
            descriptor_stat = os.fstat(file_descriptor)
            directory_stat = os.stat(report.parent)
            if (
                descriptor_stat.st_dev == directory_stat.st_dev
                and descriptor_stat.st_ino == directory_stat.st_ino
            ):
                raise OSError(errno.EINVAL, "directory fsync unsupported")
            return real_fsync(file_descriptor)

        with (
            mock.patch.object(os, "fsync", side_effect=fail_directory_fsync),
            self.assertRaises(OSError),
        ):
            history.write_report(report, "complete\n")

    def test_fsyncs_parent_of_new_report_directories(self):
        report = Path(self.temporary_directory.name) / "new" / "nested" / "report.md"
        root = Path(self.temporary_directory.name)
        real_fsync = os.fsync
        synced = set()

        def record_fsync(file_descriptor):
            descriptor_stat = os.fstat(file_descriptor)
            for directory in (root, root / "new", root / "new" / "nested"):
                if directory.exists():
                    directory_stat = os.stat(directory)
                    if (
                        descriptor_stat.st_dev == directory_stat.st_dev
                        and descriptor_stat.st_ino == directory_stat.st_ino
                    ):
                        synced.add(directory)
            return real_fsync(file_descriptor)

        with mock.patch.object(os, "fsync", side_effect=record_fsync):
            history.write_report(report, "complete\n")

        self.assertEqual(synced, {root, root / "new", root / "new" / "nested"})

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
                git_executable=self.git_executable,
                git_executable_sha256=self.git_executable_sha256,
                git_https_helper_sha256=self.git_https_helper_sha256,
            )

    def test_rejects_unreachable_final_commit(self):
        with self.assertRaises(history.InventoryError):
            history.collect_inventory(
                self.repository,
                "0" * 40,
                str(self.repository),
                allow_noncanonical_remote=True,
                git_executable=self.git_executable,
                git_executable_sha256=self.git_executable_sha256,
                git_https_helper_sha256=self.git_https_helper_sha256,
            )

    def test_rejects_credential_bearing_remote_url(self):
        with self.assertRaises(history.InventoryError):
            history.collect_inventory(
                self.repository,
                self.final_commit,
                "https://token@example.test/repository.git",
                git_executable=self.git_executable,
                git_executable_sha256=self.git_executable_sha256,
                git_https_helper_sha256=self.git_https_helper_sha256,
            )

    def test_rejects_configured_remote_name(self):
        with self.assertRaises(history.InventoryError):
            history.collect_inventory(
                self.repository,
                self.final_commit,
                "origin",
                git_executable=self.git_executable,
                git_executable_sha256=self.git_executable_sha256,
                git_https_helper_sha256=self.git_https_helper_sha256,
            )

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
                git_executable=self.git_executable,
                git_executable_sha256=self.git_executable_sha256,
                git_https_helper_sha256=self.git_https_helper_sha256,
            )

    def test_rejects_repository_local_url_rewrites(self):
        self.git(
            "config",
            "url.https://example.test/.insteadOf",
            "https://github.com/",
        )

        with self.assertRaises(history.InventoryError):
            self.inventory()

    def test_rejects_remote_name_that_shadows_the_canonical_url(self):
        self.git(
            "config",
            f"remote.{history.CANONICAL_REMOTE_URL}.url",
            str(self.repository),
        )

        with self.assertRaises(history.InventoryError):
            self.inventory()

    def test_rejects_worktree_config_scope(self):
        self.git("config", "extensions.worktreeConfig", "true")

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

    def test_git_environment_does_not_inherit_loader_or_credential_variables(self):
        with mock.patch.dict(
            os.environ,
            {
                "LD_PRELOAD": "/tmp/injected.so",
                "DYLD_INSERT_LIBRARIES": "/tmp/injected.dylib",
                "AWS_SECRET_ACCESS_KEY": "secret",
            },
        ):
            environment = history.git_environment()

        self.assertNotIn("LD_PRELOAD", environment)
        self.assertNotIn("DYLD_INSERT_LIBRARIES", environment)
        self.assertNotIn("AWS_SECRET_ACCESS_KEY", environment)

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

    def test_keeps_raw_path_identity_and_classification_after_redaction(self):
        secret_path = b"password=supersecret.wasm"
        literal_path = b"REDACTED_assigned-secret.wasm"

        secret_record = history.path_record(secret_path)
        literal_record = history.path_record(literal_path)

        self.assertEqual(secret_record["identity"], secret_path)
        self.assertEqual(literal_record["identity"], literal_path)
        self.assertTrue(secret_record["generated_candidate"])
        self.assertTrue(literal_record["generated_candidate"])

    def test_escapes_control_characters_in_report_cells(self):
        self.assertEqual(
            history.markdown("tab\treturn\rend"), "tab\\\\u0009return\\\\u000dend"
        )

    def test_escapes_markdown_links_and_images(self):
        rendered = history.markdown("![remote](https://example.test/pixel)")

        self.assertEqual(
            rendered, "\\!\\[remote\\]\\(https:\\/\\/example.test/pixel\\)"
        )

    def test_escapes_markdown_code_spans(self):
        self.assertEqual(history.markdown("`code`"), "\\`code\\`")

    def test_inline_code_uses_a_safe_backtick_delimiter(self):
        rendered = history.inline_code("value ` ![remote](https://example.test/pixel)")

        self.assertTrue(rendered.startswith("``"))
        self.assertTrue(rendered.endswith("``"))
        self.assertIn("value ` ![remote](https://example.test/pixel)", rendered)

    def test_inline_code_preserves_ampersands(self):
        self.assertEqual(history.inline_code("/tmp/a&b/git"), "`/tmp/a&b/git`")

    def test_redacts_secret_bearing_ref_for_display(self):
        ref_name = b"refs/heads/AKIA1234567890ABCDEF"
        self.git("update-ref", ref_name.decode(), self.final_commit)

        inventory = self.inventory()
        report = history.render_report(inventory, "inventory command")
        ref = next(
            item for item in inventory["refs"] if item["name"] == history.encode_ref(ref_name)
        )

        self.assertNotIn("AKIA1234567890ABCDEF", ref["display_name"])
        self.assertNotIn("AKIA1234567890ABCDEF", report)
        self.assertIn(hashlib.sha256(ref_name).hexdigest(), ref["display_name"])
        self.assertEqual(history.encode_ref(ref_name), "refs/heads/AKIA1234567890ABCDEF")

    def test_pinned_git_runner_ignores_inherited_path_and_exec_path(self):
        fake_directory = Path(self.temporary_directory.name) / "fake-bin"
        fake_directory.mkdir()
        fake_git = fake_directory / "git"
        fake_git.write_text("#!/bin/sh\nexit 99\n")
        fake_git.chmod(0o755)

        with mock.patch.dict(
            os.environ,
            {"PATH": str(fake_directory), "GIT_EXEC_PATH": str(fake_directory)},
        ):
            output = self.runner.limited(self.repository, 128, "--version")

        self.assertTrue(output.startswith(b"git version "))

    def test_pinned_git_runner_rejects_snapshot_mutation(self):
        runner = history.GitRunner(
            self.git_executable,
            self.git_executable_sha256,
            self.git_https_helper_sha256,
        )
        executable = Path(runner.command)
        original = executable.read_bytes()
        try:
            executable.chmod(0o700)
            executable.write_bytes(original + b"altered")
            with self.assertRaisesRegex(history.InventoryError, "snapshot changed"):
                runner.close()
        finally:
            if executable.exists():
                executable.write_bytes(original)
                executable.chmod(0o500)
                runner.close()

    def test_rejects_unexpected_git_program_digest(self):
        with self.assertRaisesRegex(history.InventoryError, "does not match"):
            history.GitRunner(
                self.git_executable, "0" * 64, self.git_https_helper_sha256
            )

    def test_rejects_relative_git_executable(self):
        with self.assertRaises(history.InventoryError):
            history.GitRunner(
                "git", self.git_executable_sha256, self.git_https_helper_sha256
            )

    def test_object_reads_use_the_bounded_runner(self):
        object_id = self.git("hash-object", "src.txt").stdout.strip()
        size = int(self.git("cat-file", "-s", object_id).stdout)

        with mock.patch.object(
            self.runner, "limited", wraps=self.runner.limited
        ) as limited:
            data = history.read_git_object(
                self.repository, "blob", object_id, size, runner=self.runner
            )

        self.assertEqual(data, b"current\n")
        limited.assert_called_once_with(
            self.repository, size, "cat-file", "blob", object_id, input_data=None
        )

    def test_rejects_object_output_above_declared_size(self):
        object_id = self.git("hash-object", "src.txt").stdout.strip()

        with self.assertRaises(history.InventoryError):
            history.read_git_object(
                self.repository, "blob", object_id, 1, runner=self.runner
            )

    def test_rejects_object_content_that_does_not_match_the_object_id(self):
        object_id = self.git("hash-object", "src.txt").stdout.strip()
        size = int(self.git("cat-file", "-s", object_id).stdout)
        replacement = b"changed\n"
        self.assertEqual(len(replacement), size)

        with (
            mock.patch.object(self.runner, "limited", return_value=replacement),
            self.assertRaisesRegex(history.InventoryError, "object ID mismatch"),
        ):
            history.read_git_object(
                self.repository, "blob", object_id, size, runner=self.runner
            )

    def test_object_read_timeout_kills_the_git_process(self):
        runner = mock.Mock()
        runner.limited.side_effect = lambda *args, **kwargs: history.run_limited_process(
            ["/bin/sleep", "10"], history.git_environment(), args[1]
        )

        with (
            mock.patch.object(history, "GIT_TIMEOUT_SECONDS", 0.01),
            self.assertRaisesRegex(history.InventoryError, "timed out"),
        ):
            history.read_git_object(
                self.repository, "blob", "1" * 40, 1, runner=runner
            )

    def test_output_limit_does_not_wait_for_git_eof(self):
        runner = mock.Mock()
        runner.limited.side_effect = lambda *args, **kwargs: history.run_limited_process(
            ["/bin/sh", "-c", "printf ab; exec /bin/sleep 10"],
            history.git_environment(),
            args[1],
        )

        with (
            mock.patch.object(history, "GIT_TIMEOUT_SECONDS", 5),
            self.assertRaisesRegex(history.InventoryError, "output exceeded 1 bytes"),
        ):
            history.read_git_object(
                self.repository, "blob", "1" * 40, 1, runner=runner
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
                runner=self.runner,
            )

    def test_rejects_non_posix_platform_before_spawn(self):
        with (
            mock.patch.object(history.os, "name", "nt"),
            mock.patch.object(history.subprocess, "Popen") as popen,
            self.assertRaises(history.InventoryError),
        ):
            history.git_limited(self.repository, 1, "status", runner=self.runner)

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
            history.tree_entries(
                self.repository, self.final_commit, runner=self.runner
            )

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
            history.git_text(self.repository, "status", runner=self.runner)

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
                git_executable=self.git_executable,
                git_executable_sha256=self.git_executable_sha256,
                git_https_helper_sha256=self.git_https_helper_sha256,
            )


if __name__ == "__main__":
    unittest.main()
