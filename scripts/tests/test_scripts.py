"""Tests for the release scripts.

The rules these cover are the ones that fail quietly: a leaked credential in
published notes, or a tag that names the wrong crate. Neither breaks the build,
so nothing else would catch them.
"""

import importlib.util
import sys
import unittest
from pathlib import Path

SCRIPTS = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(SCRIPTS))


def load(name: str):
    spec = importlib.util.spec_from_file_location(name, SCRIPTS / f"{name}.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


publish = load("publish")
release_notes = load("release_notes")
check_token = load("check_token")


class RepoUrl(unittest.TestCase):
    """`repo_url` feeds links that are published on the release page."""

    def run_with_remote(self, url: str) -> str:
        original = release_notes.git

        def fake(*args: str) -> str:
            return url + "\n"

        release_notes.git = fake
        try:
            return release_notes.repo_url()
        finally:
            release_notes.git = original

    def test_strips_a_token_from_an_https_remote(self):
        self.assertEqual(
            self.run_with_remote(
                "https://x-access-token:ghs_SECRETVALUE@github.com/cipansor/crateworks.git"
            ),
            "https://github.com/cipansor/crateworks",
        )

    def test_strips_a_username_and_password(self):
        self.assertEqual(
            self.run_with_remote("https://user:pw@github.com/o/r.git"),
            "https://github.com/o/r",
        )

    def test_converts_an_ssh_remote(self):
        self.assertEqual(
            self.run_with_remote("git@github.com:cipansor/crateworks.git"),
            "https://github.com/cipansor/crateworks",
        )

    def test_plain_https_remote_is_unchanged(self):
        self.assertEqual(
            self.run_with_remote("https://github.com/cipansor/crateworks.git"),
            "https://github.com/cipansor/crateworks",
        )

    def test_no_credentials_survive_in_any_form(self):
        for url in (
            "https://x-access-token:ghs_ABC@github.com/o/r.git",
            "https://token@github.com/o/r.git",
            "https://a:b@github.com/o/r",
        ):
            with self.subTest(url=url):
                out = self.run_with_remote(url)
                self.assertNotIn("@", out)
                self.assertNotIn("ghs_", out)


class CratesFromTag(unittest.TestCase):
    """A release tag names one crate; anything else must be refused."""

    def test_parses_a_crate_tag(self):
        self.assertEqual(publish.crates_from_tag("xmlkit-v0.1.1"), ("xmlkit", "0.1.1"))

    def test_rejects_a_bare_version_tag(self):
        with self.assertRaises(publish.ReleaseError):
            publish.crates_from_tag("v0.1.1")

    def test_rejects_a_tag_with_no_version(self):
        with self.assertRaises(publish.ReleaseError):
            publish.crates_from_tag("xmlkit")

    def test_rejects_an_empty_tag(self):
        with self.assertRaises(publish.ReleaseError):
            publish.crates_from_tag("")


class ManifestVersions(unittest.TestCase):
    """The tag has to agree with the manifest, so reading it must be right."""

    def test_every_crate_declares_a_version(self):
        crates, _, manifests = publish.load_manifests()
        self.assertGreater(len(crates), 0)
        for crate in crates:
            with self.subTest(crate=crate):
                self.assertTrue(publish.version_of(manifests[crate]))

    def test_workspace_inheritance_resolves_to_the_workspace_version(self):
        _, _, manifests = publish.load_manifests()
        # xmlkit inherits `version.workspace = true`; a naive read of the raw
        # manifest would return a dict instead of a version string.
        self.assertIsInstance(publish.version_of(manifests["xmlkit"]), str)


class PreviousTag(unittest.TestCase):
    """Notes must range from the previous tag of the same crate."""

    def test_version_key_sorts_numerically(self):
        self.assertLess(
            release_notes.version_key("c-v0.1.9", "c"),
            release_notes.version_key("c-v0.1.10", "c"),
        )

    def test_version_key_ignores_the_crate_name(self):
        self.assertEqual(
            release_notes.version_key("a-v1.2.3", "a"),
            release_notes.version_key("b-v1.2.3", "b"),
        )


class Grouping(unittest.TestCase):
    """Nothing may be dropped from the notes, however the subject is written."""

    def test_unknown_type_lands_in_other(self):
        grouped = release_notes.group([("abc1234", "Bump the MSRV floor")])
        self.assertEqual(grouped[release_notes.OTHER], [("abc1234", "Bump the MSRV floor")])

    def test_known_types_are_sectioned(self):
        grouped = release_notes.group([("abc1234", "feat: add a parser")])
        self.assertEqual(grouped["Features"], [("abc1234", "add a parser")])

    def test_scope_and_bang_are_handled(self):
        grouped = release_notes.group([("abc1234", "fix(xmlkit)!: reject bad input")])
        self.assertEqual(grouped["Fixes"], [("abc1234", "reject bad input")])

    def test_no_commit_is_lost(self):
        commits = [
            ("a", "feat: one"),
            ("b", "chore: two"),
            ("c", "no convention here"),
            ("d", "weird: type"),
        ]
        grouped = release_notes.group(commits)
        self.assertEqual(sum(len(v) for v in grouped.values()), len(commits))


class TokenCheck(unittest.TestCase):
    """`check_token` tells a usable token from a rejected one by the body.

    Both answers are HTTP 403, so a status-code check would call a bad token
    good. These pin the distinction.
    """

    def test_a_web_session_answer_means_the_token_is_usable(self):
        ok, _ = check_token.classify(
            403, '{"errors":[{"detail":"this action can only be performed on the crates.io website"}]}'
        )
        self.assertTrue(ok)

    def test_authentication_failure_means_the_token_is_not(self):
        ok, _ = check_token.classify(403, '{"errors":[{"detail":"authentication failed"}]}')
        self.assertFalse(ok)

    def test_ok_status_is_usable(self):
        ok, _ = check_token.classify(200, "{}")
        self.assertTrue(ok)

    def test_an_unknown_answer_is_not_treated_as_success(self):
        ok, message = check_token.classify(500, "gateway blew up")
        self.assertFalse(ok)
        self.assertIn("500", message)


class NoSecretsInOutput(unittest.TestCase):
    """The scripts are run in CI, where stdout is public."""

    def test_check_token_never_prints_the_token(self):
        token = "cio_SECRET_VALUE_aaaaaaaaaaaaaaaaaaaa"
        source = (SCRIPTS / "check_token.py").read_text()
        for pattern in ("print(token", "print(f\"{token}", "repr(token)"):
            with self.subTest(pattern=pattern):
                self.assertNotIn(pattern, source)
        # The classifier returns a message, never the credential it was given.
        _, message = check_token.classify(403, '{"errors":[{"detail":"authentication failed"}]}')
        self.assertNotIn(token, message)


if __name__ == "__main__":
    unittest.main()
