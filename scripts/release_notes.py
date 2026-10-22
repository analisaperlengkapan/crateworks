#!/usr/bin/env python3
"""Generate release notes for one crate from the commits that touched it.

A release tag names one crate (`xmlkit-v0.1.1`), so the notes for that release
should describe that crate and nothing else. A repository-wide changelog would
mix seventeen crates together; these notes are scoped to the crate's directory.

The range runs from the previous tag of the same crate to the tag being
released, so a crate that skipped a release does not repeat notes it already
published. With no previous tag for the crate this is its first release and the
crate's whole history is used.

Usage:
    python3 scripts/release_notes.py --tag xmlkit-v0.1.1
    python3 scripts/release_notes.py --tag xmlkit-v0.1.1 --out notes.md
"""

import argparse
import re
import subprocess
import sys
from pathlib import Path

# `publish` is a sibling script, not an installed package. Run as a file its
# directory is already on the path; imported from elsewhere it is not, and this
# keeps that working so the module can be tested without cd-ing first.
sys.path.insert(0, str(Path(__file__).resolve().parent))

import publish

ROOT = Path(__file__).resolve().parent.parent

# Conventional-commit types, in the order the sections are printed. A subject
# that does not follow the convention is not dropped: it lands in "Other".
SECTIONS = [
    ("feat", "Features"),
    ("fix", "Fixes"),
    ("perf", "Performance"),
    ("refactor", "Refactoring"),
    ("docs", "Documentation"),
    ("test", "Tests"),
    ("build", "Build"),
    ("ci", "CI"),
    ("chore", "Maintenance"),
    ("revert", "Reverts"),
]
SECTION_ORDER = [title for _, title in SECTIONS]
SECTION_FOR = dict(SECTIONS)
OTHER = "Other changes"

CONVENTIONAL = re.compile(
    r"^(?P<type>[a-z]+)(?:\((?P<scope>[^)]*)\))?(?P<breaking>!)?: (?P<subject>.+)$"
)


def git(*args: str) -> str:
    result = subprocess.run(
        ["git", *args], cwd=ROOT, capture_output=True, text=True
    )
    if result.returncode != 0:
        raise SystemExit(f"git {' '.join(args)} failed: {result.stderr.strip()}")
    return result.stdout


def require_full_history() -> None:
    """Refuse to run on a shallow clone.

    The commit range comes from `git log` and the previous tag from `git tag`, so
    a shallow checkout silently produces notes for the wrong range - usually "no
    changes" - instead of failing. `actions/checkout` is shallow by default, so
    this is the normal mistake, and it is caught here rather than published.
    """
    if git("rev-parse", "--is-shallow-repository").strip() == "true":
        raise SystemExit(
            "shallow clone: the commit range and tags are incomplete. Fetch full "
            "history first (actions/checkout with `fetch-depth: 0`)."
        )


def repo_url() -> str:
    """The repository's web URL, for commit and compare links."""
    url = git("remote", "get-url", "origin").strip()
    if url.startswith("git@github.com:"):
        url = "https://github.com/" + url.removeprefix("git@github.com:")
    return url.removesuffix(".git")


def crate_tags(crate: str) -> list[str]:
    out = git("tag", "--list", f"{crate}-v*")
    return [line.strip() for line in out.splitlines() if line.strip()]


def version_key(tag: str, crate: str) -> tuple[int, ...]:
    """Sort tags by the numbers in the version, so 0.1.10 follows 0.1.9."""
    numbers = re.findall(r"\d+", tag.removeprefix(f"{crate}-v"))
    return tuple(int(n) for n in numbers)


def is_ancestor(rev: str) -> bool:
    result = subprocess.run(
        ["git", "merge-base", "--is-ancestor", rev, "HEAD"],
        cwd=ROOT,
        capture_output=True,
    )
    return result.returncode == 0


def previous_tag(crate: str, current: str) -> str | None:
    """The highest earlier tag of this crate that HEAD descends from."""
    candidates = [
        tag for tag in crate_tags(crate) if tag != current and is_ancestor(tag)
    ]
    if not candidates:
        return None
    return max(candidates, key=lambda tag: version_key(tag, crate))


def crate_commits(crate: str, since: str | None) -> list[tuple[str, str]]:
    """(short hash, subject) for commits touching the crate, newest first."""
    args = ["log", "--no-merges", "--format=%h%x1f%s"]
    if since:
        args.append(f"{since}..HEAD")
    args += ["--", f"crates/{crate}"]
    out = git(*args)
    return [
        tuple(line.split("\x1f", 1))  # type: ignore[misc]
        for line in out.splitlines()
        if "\x1f" in line
    ]


def group(commits: list[tuple[str, str]]) -> dict[str, list[tuple[str, str]]]:
    grouped: dict[str, list[tuple[str, str]]] = {}
    for short, subject in commits:
        match = CONVENTIONAL.match(subject)
        if match:
            title = SECTION_FOR.get(match.group("type"), OTHER)
            text = match.group("subject")
        else:
            title = OTHER
            text = subject
        grouped.setdefault(title, []).append((short, text))
    return grouped


def render(crate: str, version: str, tag: str) -> str:
    previous = previous_tag(crate, tag)
    commits = crate_commits(crate, previous)
    grouped = group(commits)
    url = repo_url()

    lines = [f"# {crate} {version}", ""]
    if previous:
        lines.append(f"Changes since `{previous}`.")
    else:
        lines.append("First release.")
    lines.append("")

    for title in [*SECTION_ORDER, OTHER]:
        entries = grouped.get(title)
        if not entries:
            continue
        lines.append(f"### {title}")
        for short, text in entries:
            lines.append(f"- {text} ([{short}]({url}/commit/{short}))")
        lines.append("")

    if not commits:
        lines.append("No changes to this crate in the range.")
        lines.append("")

    if previous:
        lines.append(f"**Full changelog**: {url}/compare/{previous}...{tag}")
    else:
        lines.append(f"**Commits**: {url}/commits/{tag}")
    lines.append("")
    lines.append(f"crates.io: https://crates.io/crates/{crate}")
    lines.append(f"docs.rs: https://docs.rs/{crate}/{version}")
    return "\n".join(lines).rstrip() + "\n"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--tag", required=True, help="release tag, e.g. xmlkit-v0.1.1")
    parser.add_argument("--out", help="write here instead of stdout")
    args = parser.parse_args()

    require_full_history()

    try:
        crate, version = publish.crates_from_tag(args.tag)
    except publish.ReleaseError as error:
        print(f"error: {error}", file=sys.stderr)
        return 1

    crates, _, _ = publish.load_manifests()
    if crate not in crates:
        print(f"error: `{crate}` is not a crate here", file=sys.stderr)
        return 1

    notes = render(crate, version, args.tag)
    if args.out:
        Path(args.out).write_text(notes)
        print(f"wrote {args.out}")
    else:
        print(notes, end="")
    return 0


if __name__ == "__main__":
    sys.exit(main())
