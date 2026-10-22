#!/usr/bin/env python3
"""Check, and optionally run, the crateworks publishing sequence.

`cargo publish` resolves a `path` dependency against the registry, so a crate
that depends on another crate in this workspace cannot be packaged until that
dependency is published. Publishing in the wrong order fails with

    no matching package named `xmlkit` found

which reads like a broken manifest but is only the order. This script derives the
order from the manifests themselves, so it cannot go stale the way a hand-written
list does, and runs the crates in it.

By default it only checks: `cargo publish --dry-run` for every crate, in order,
stopping at nothing, reporting which are ready and which are waiting on a
dependency. It uploads nothing. Pass --publish to upload for real; that path
stops at the first failure, because a published version cannot be overwritten and
there is no point continuing past a broken crate.

Usage:
    python3 scripts/publish.py                 # check every crate
    python3 scripts/publish.py --strict        # check, and fail if any is blocked
    python3 scripts/publish.py --publish       # upload, in order, stop on error
    python3 scripts/publish.py --publish --yes # same, no confirmation prompt

A rate limit from crates.io is waited out and retried rather than reported as a
broken crate: the 429 names the moment to retry and that time is honoured, since
the new-crate window is minutes long. A version that is already on the registry
is reported as such and skipped, so a re-run after a partial release finishes the
rest instead of failing on the crates it already uploaded.

`cargo publish --workspace` also orders by dependency and is what to reach for
when cargo's own output is wanted. This script exists for the check, which that
command cannot do without attempting an upload, and for the summary.

Exit code is 0 when nothing is wrong: a crate waiting on an unpublished
dependency is the expected pre-publish state, not an error. Pass --strict to make
it one, which is the check to run after publishing a dependency.
"""

import argparse
import re
import subprocess
import sys
import time
import tomllib
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

# "no matching package named `xmlkit` found"
MISSING_DEP = re.compile(r"no matching package named `([^`]+)` found")

# crates.io rate limits how many NEW crates an account may publish in a short
# window (about one every ten minutes), and a second run of the same version is
# refused outright. Cargo surfaces both as a plain non-zero exit, so they are
# detected here rather than reported as a broken crate.
#
# The 429 names the exact moment to retry:
#   ... Please try again after Thu, 22 Oct 2026 03:32:25 GMT ...
# so that timestamp is honoured instead of guessing a delay; a fixed 70s backoff
# is far too short for the new-crate limit and just burns attempts.
RATE_LIMITED = re.compile(r"429|too many requests|rate limit", re.IGNORECASE)
RETRY_AFTER = re.compile(
    r"try again after (\w{3}, \d{2} \w{3} \d{4} \d{2}:\d{2}:\d{2} GMT)"
)
ALREADY_PUBLISHED = re.compile(r"already exists on crates\.io")
RATE_LIMIT_FALLBACK = 70  # seconds, used only when the 429 carries no timestamp


def rate_limit_wait(output: str) -> float:
    """Seconds to wait before retrying a rate-limited upload.

    Reads the retry time out of the 429 when it is there, because the new-crate
    window is minutes long and a fixed short backoff cannot clear it.
    """
    match = RETRY_AFTER.search(output)
    if not match:
        return float(RATE_LIMIT_FALLBACK)
    try:
        when = datetime.strptime(match.group(1), "%a, %d %b %Y %H:%M:%S GMT")
    except ValueError:
        return float(RATE_LIMIT_FALLBACK)
    return max(65.0, when.replace(tzinfo=timezone.utc).timestamp() - time.time() + 5)



def load_manifests() -> tuple[list[str], dict[str, list[str]]]:
    """Return (member dirs, member name -> workspace member deps)."""
    workspace = tomllib.loads((ROOT / "Cargo.toml").read_text())
    members = [m for m in workspace["workspace"]["members"] if m.startswith("crates/")]

    manifests: dict[str, dict] = {}
    names: dict[str, str] = {}
    for member in members:
        parsed = tomllib.loads((ROOT / member / "Cargo.toml").read_text())
        name = parsed["package"]["name"]
        manifests[member] = parsed
        names[name] = member

    deps: dict[str, list[str]] = {}
    for member, parsed in manifests.items():
        name = parsed["package"]["name"]
        deps[name] = [
            dep for dep in parsed.get("dependencies", {}) if dep in names
        ]
    return [manifests[m]["package"]["name"] for m in members], deps


def topo_order(crates: list[str], deps: dict[str, list[str]]) -> list[str]:
    """Order crates so every crate follows the ones it depends on."""
    ordered: list[str] = []
    placed: set[str] = set()
    while len(ordered) < len(crates):
        progressed = False
        for crate in crates:
            if crate in placed:
                continue
            if all(dep in placed for dep in deps[crate]):
                ordered.append(crate)
                placed.add(crate)
                progressed = True
        if not progressed:
            raise SystemExit(f"dependency cycle among: {sorted(set(crates) - placed)}")
    return ordered


def dirty_files() -> list[str]:
    """Changed or untracked files, excluding `.openhands/` which is never committed.

    cargo's own check is scoped to the crate directory: a dirty file at the repo
    root does not stop `cargo publish -p xmlkit`, but one under
    `crates/xmlkit/` does. So this returns every dirty path and the caller decides
    which crates each one blocks, rather than warning about all of them.
    """
    result = subprocess.run(
        ["git", "status", "--porcelain"],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    return [
        line
        for line in result.stdout.splitlines()
        if line.strip() and ".openhands/" not in line
    ]


def dirty_by_crate(dirty: list[str], crates: list[str]) -> dict[str, list[str]]:
    """Attribute each dirty file to the crate whose directory contains it."""
    blocked: dict[str, list[str]] = {}
    for line in dirty:
        path = line[3:].strip().strip('"')
        for crate in crates:
            if path.startswith(f"crates/{crate}/"):
                blocked.setdefault(crate, []).append(path)
    return blocked


def run_publish(crate: str, dry_run: bool) -> tuple[str, str]:
    """Return (verdict, detail).

    Verdict is 'ready', 'blocked' (a dependency is unpublished), 'dirty' (files
    in the crate are uncommitted) or 'failed'.
    """
    command = ["cargo", "publish", "-p", crate]
    if dry_run:
        command.append("--dry-run")
    result = subprocess.run(command, cwd=ROOT, capture_output=True, text=True)
    output = result.stdout + result.stderr

    if result.returncode == 0:
        return "ready", ""

    if "not yet committed into git" in output or "--allow-dirty" in output:
        paths = [
            line.strip()
            for line in output.splitlines()
            if line.strip().startswith("crates/") or line.strip().endswith(".rs")
        ]
        return "dirty", paths[0] if paths else "uncommitted files"

    missing = MISSING_DEP.search(output)
    if missing:
        return "blocked", missing.group(1)

    if ALREADY_PUBLISHED.search(output):
        return "already-published", ""

    if RATE_LIMITED.search(output):
        # The whole message is the detail here: the caller parses the retry time
        # out of it.
        return "rate-limited", output

    for line in output.splitlines():
        if line.startswith("error"):
            return "failed", line.removeprefix("error:").strip()
    return "failed", f"exit code {result.returncode}"


def publish_with_retry(crate: str, attempts: int = 12) -> tuple[str, str]:
    """Publish one crate, waiting out crates.io rate limiting.

    A rate limit is not a problem with the crate, so it is retried instead of
    being reported, for as long as the 429 keeps naming a retry time. The wait is
    read from that message, because the new-crate window is minutes long and a
    fixed short backoff cannot clear it. Any other verdict is returned as-is.
    """
    for attempt in range(1, attempts + 1):
        verdict, detail = run_publish(crate, dry_run=False)
        if verdict != "rate-limited":
            return verdict, detail
        if attempt == attempts:
            break
        wait = rate_limit_wait(detail) if detail else RATE_LIMIT_FALLBACK
        print(
            f"    crates.io rate limited this upload; waiting "
            f"{wait:.0f}s (attempt {attempt} of {attempts})"
        )
        time.sleep(wait)
    return "failed", "still rate limited after retries"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument(
        "--publish", action="store_true", help="upload for real instead of dry-running"
    )
    parser.add_argument(
        "--yes", action="store_true", help="skip the confirmation before uploading"
    )
    parser.add_argument(
        "--strict", action="store_true", help="treat a blocked crate as a failure"
    )
    parser.add_argument(
        "--delay",
        type=float,
        default=0.0,
        metavar="SECONDS",
        help="wait this long between uploads (crates.io allows 1 per minute)",
    )
    args = parser.parse_args()

    crates, deps = load_manifests()
    order = topo_order(crates, deps)

    if not args.publish:
        dirty = dirty_by_crate(dirty_files(), crates)
        if dirty:
            print(
                "warning: uncommitted changes inside these crates, so cargo will "
                "refuse to package them:",
                file=sys.stderr,
            )
            for crate in sorted(dirty):
                for path in dirty[crate]:
                    print(f"  {crate}: {path}", file=sys.stderr)
            print(file=sys.stderr)

    if args.publish:
        print(f"About to upload {len(order)} crates to crates.io, in this order:\n")
        for index, crate in enumerate(order, 1):
            waiting = f"  (needs {', '.join(deps[crate])})" if deps[crate] else ""
            print(f"  {index:2}. {crate}{waiting}")
        print(
            "\nA published version cannot be overwritten or deleted. If a crate "
            "here is wrong, its next release has to carry a new version number."
        )
        if not args.yes:
            try:
                answer = input("\nType 'publish' to continue: ").strip()
            except EOFError:
                # No terminal, or stdin closed. Silence is not consent.
                answer = ""
            if answer != "publish":
                print("aborted; nothing was uploaded")
                return 1

    width = max(len(c) for c in order)
    blocked: list[tuple[str, str]] = []
    dirty: list[tuple[str, str]] = []
    failed: list[tuple[str, str]] = []
    ready: list[str] = []
    already: list[str] = []
    uploaded_any = False

    for crate in order:
        if args.publish:
            if uploaded_any and args.delay:
                time.sleep(args.delay)
            verdict, detail = publish_with_retry(crate)
            if verdict != "failed":
                uploaded_any = True
        else:
            verdict, detail = run_publish(crate, dry_run=True)
        if verdict == "ready":
            ready.append(crate)
            print(f"{crate:<{width}}  ready")
        elif verdict == "already-published":
            already.append(crate)
            print(f"{crate:<{width}}  already published")
        elif verdict == "blocked":
            blocked.append((crate, detail))
            print(f"{crate:<{width}}  blocked, needs {detail} published first")
        elif verdict == "dirty":
            dirty.append((crate, detail))
            print(f"{crate:<{width}}  uncommitted: {detail}")
        else:
            failed.append((crate, detail))
            print(f"{crate:<{width}}  FAILED: {detail}")

        if verdict == "failed" and args.publish:
            print("\nstopped at the first failure; nothing after it was attempted")
            break

    summary = f"\n{len(ready)} ready, {len(blocked)} blocked on order"
    if already:
        summary += f", {len(already)} already published"
    if dirty:
        summary += f", {len(dirty)} with uncommitted changes"
    summary += f", {len(failed)} failed"
    print(summary)

    if blocked:
        needs = sorted({detail for _, detail in blocked})
        print(f"Publish {', '.join(needs)} first, then run this again.")
    if dirty:
        print("Commit the changes above; cargo will not package a dirty crate.")
    if failed:
        print("Fix the failures above before publishing anything.", file=sys.stderr)
        return 1
    if args.strict and (blocked or dirty):
        print("--strict: a blocked or dirty crate is a failure here.", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
