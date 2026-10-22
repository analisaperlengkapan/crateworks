#!/usr/bin/env python3
"""Check that the crates.io token in the environment is one the registry accepts.

`cargo publish` reports a bad token only when it gets to the upload, which is
after the dry run and partway through a release. This asks the registry first so
a missing or expired secret fails immediately, with a message that says so.

The status code cannot be used to tell the two apart: `/api/v1/me` answers 403
for an authenticated token too, because it is a web-session endpoint. The body
is what differs - a token the registry knows gets "this action can only be
performed on the crates.io website", one it does not gets "authentication
failed". The token itself is never printed.
"""

import json
import os
import sys
import urllib.error
import urllib.request

ME = "https://crates.io/api/v1/me"

WEB_SESSION_ONLY = "this action can only be performed on the crates.io website"
AUTHENTICATION_FAILED = "authentication failed"


def classify(status: int, body: str) -> tuple[bool, str]:
    """Decide whether a response to `/api/v1/me` means the token is usable."""
    if status == 200:
        return True, "the registry accepted the token"
    if WEB_SESSION_ONLY in body:
        return True, "the registry accepted the token (this endpoint needs a web session)"
    if AUTHENTICATION_FAILED in body:
        return False, "the registry rejected the token as invalid or expired"
    return False, f"unexpected answer from the registry: HTTP {status}"


def check(token: str) -> tuple[bool, str]:
    request = urllib.request.Request(ME, headers={"User-Agent": "crateworks-release"})
    request.add_header("Authorization", token)
    try:
        with urllib.request.urlopen(request, timeout=30) as response:
            return classify(response.status, response.read().decode())
    except urllib.error.HTTPError as error:
        return classify(error.code, error.read().decode())
    except Exception as error:
        return False, f"could not reach the registry: {error}"


def main() -> int:
    token = os.environ.get("CARGO_REGISTRY_TOKEN", "").strip()
    if not token:
        print("CARGO_REGISTRY_TOKEN is empty or unset.", file=sys.stderr)
        print("Add it under Settings -> Secrets and variables -> Actions.", file=sys.stderr)
        return 1

    ok, message = check(token)
    print(("OK: " if ok else "FAILED: ") + message)
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
