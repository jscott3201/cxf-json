import copy
import importlib.util
import json
import os
import tempfile
import unittest
from pathlib import Path


module_path = Path(__file__).with_name("check-producer-observations.py")
spec = importlib.util.spec_from_file_location("check_producer_observations", module_path)
if spec is None or spec.loader is None:
    raise RuntimeError("failed to load producer observation checker")
observations = importlib.util.module_from_spec(spec)
spec.loader.exec_module(observations)


class ProducerObservationTests(unittest.TestCase):
    def setUp(self):
        self.temporary_directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary_directory.cleanup)
        self.root = Path(self.temporary_directory.name)
        self.witness = (
            self.root
            / "crates"
            / "cxf-json"
            / "tests"
            / "projection"
            / "case.jsonld"
        )
        self.witness.parent.mkdir(parents=True)
        self.witness.write_text("{}\n")
        self.manifest = self.root / "manifest.json"
        self.document = {
            "schema_version": 1,
            "observations": [
                {
                    "id": "producer-v1.0.0",
                    "producer_repository": "https://github.com/example/producer",
                    "producer_commit": "a" * 40,
                    "producer_release": "v1.0.0",
                    "dialect": {
                        "namespace_iri": "http://data.ashrae.org/S231#",
                        "connection_predicate": "http://data.ashrae.org/S231#connectedTo",
                    },
                    "evidence": {
                        "class": "producer-source",
                        "url": "https://raw.githubusercontent.com/example/producer/"
                        + "a" * 40
                        + "/producer.js",
                    },
                    "witnesses": {
                        "namespace_iri": "crates/cxf-json/tests/projection/case.jsonld",
                        "connection_predicate": "crates/cxf-json/tests/projection/case.jsonld",
                    },
                }
            ],
        }
        self.expected_observations = {
            "producer-v1.0.0": {
                "pin": (
                    "https://github.com/example/producer",
                    "a" * 40,
                    "v1.0.0",
                ),
                "dialect": self.document["observations"][0]["dialect"],
                "evidence": self.document["observations"][0]["evidence"],
            }
        }

    def write_manifest(self, document=None):
        self.manifest.write_text(json.dumps(document or self.document))

    def assert_rejected(self, document, message):
        self.write_manifest(document)
        with self.assertRaisesRegex(ValueError, message):
            observations.verify(self.manifest, self.root, self.expected_observations)

    def test_accepts_source_free_observation(self):
        self.write_manifest()
        observations.verify(self.manifest, self.root, self.expected_observations)

    def test_rejects_non_full_commit_and_mutable_evidence_url(self):
        document = copy.deepcopy(self.document)
        document["observations"][0]["producer_commit"] = "a" * 7
        self.assert_rejected(document, "full lowercase Git commit")

        document = copy.deepcopy(self.document)
        document["observations"][0]["evidence"]["url"] = (
            "https://github.com/example/producer/blob/main/producer.js"
        )
        self.assert_rejected(document, "immutable primary-source HTTPS URL")

    def test_rejects_unsupported_evidence_class_and_claim_fields(self):
        document = copy.deepcopy(self.document)
        document["observations"][0]["evidence"]["class"] = "generated-fixture"
        self.assert_rejected(document, "class is unsupported")

        document = copy.deepcopy(self.document)
        document["observations"][0]["compatibility"] = "passes"
        self.assert_rejected(document, "extra=\\['compatibility'\\]")

    def test_rejects_changed_dialect_fact(self):
        document = copy.deepcopy(self.document)
        document["observations"][0]["dialect"]["connection_predicate"] = (
            "http://data.ashrae.org/S231#isConnectedTo"
        )
        self.assert_rejected(document, "approved producer facts")

    def test_rejects_extra_observations_before_reading_witnesses(self):
        document = copy.deepcopy(self.document)
        document["observations"].append(copy.deepcopy(document["observations"][0]))
        self.witness.unlink()
        self.assert_rejected(document, "every approved producer observation exactly once")

    def test_rejects_oversized_witness(self):
        self.witness.write_bytes(b"x" * (observations.MAX_WITNESS_BYTES + 1))
        self.write_manifest()
        with self.assertRaisesRegex(ValueError, "file exceeds"):
            observations.verify(self.manifest, self.root, self.expected_observations)

    def test_rejects_external_and_traversing_witness_paths(self):
        document = copy.deepcopy(self.document)
        document["observations"][0]["witnesses"]["namespace_iri"] = (
            "https://example.test/case.jsonld"
        )
        self.assert_rejected(document, "owned JSON-LD witness")

        document = copy.deepcopy(self.document)
        document["observations"][0]["witnesses"]["namespace_iri"] = (
            "crates/cxf-json/tests/projection/../case.jsonld"
        )
        self.assert_rejected(document, "without parent traversal")

    @unittest.skipUnless(hasattr(os, "symlink"), "symlinks are unavailable")
    def test_rejects_symlinked_witness(self):
        target = self.root / "target.jsonld"
        target.write_text("{}\n")
        self.witness.unlink()
        self.witness.symlink_to(target)
        self.write_manifest()
        with self.assertRaisesRegex(ValueError, "without following symlinks"):
            observations.verify(self.manifest, self.root, self.expected_observations)

    def test_rejects_duplicate_json_members(self):
        self.manifest.write_text('{"schema_version":1,"schema_version":1}')
        with self.assertRaisesRegex(ValueError, "duplicate JSON member"):
            observations.verify(self.manifest, self.root, self.expected_observations)


if __name__ == "__main__":
    unittest.main()
