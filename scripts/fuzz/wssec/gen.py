"""Generates the digest and base64 fixtures for the `wssec` crate.

The crate carries its own SHA-1, MD5 and base64 because the workspace is
std-only. Hand-written hash code breaks at particular lengths, not particular
inputs: the padding spills into a second block at 56 bytes, and a length that
is a multiple of the block size has its length field in a block of its own. So
the fixture walks every length from 0 to 1024 rather than picking samples, and
the expected values come from Python's `hashlib` and `base64`, which are the
reference for both.

Run with the fixture directory as the argument:

    python3 scripts/fuzz/wssec/gen.py crates/wssec/tests/fixtures
"""

import base64
import hashlib
import pathlib
import random
import sys

out_dir = pathlib.Path(sys.argv[1])
out_dir.mkdir(parents=True, exist_ok=True)

random.seed(20261021)

# Every length to 1024 covers each block-boundary case four times over; the
# bytes are a fixed pattern so the fixture is reproducible, with a second run
# of random bytes for the lengths that matter most.
MAX = 1024


def body(length):
    return bytes((i * 7 + length) % 256 for i in range(length))


def hexdigest(algorithm, data):
    return hashlib.new(algorithm, data).hexdigest()


rows = ["length\tdata_hex\tsha1\tmd5"]
for length in range(0, MAX + 1):
    data = body(length)
    rows.append(
        f"{length}\t{data.hex()}\t{hexdigest('sha1', data)}\t{hexdigest('md5', data)}"
    )
for _ in range(256):
    data = bytes(random.randrange(256) for _ in range(random.randrange(0, MAX)))
    rows.append(
        f"{len(data)}\t{data.hex()}\t{hexdigest('sha1', data)}\t{hexdigest('md5', data)}"
    )
(out_dir / "digest_lengths.tsv").write_text("\n".join(rows) + "\n")

rows = ["data_hex\tstandard\turl_safe\tpadded"]
for length in range(0, 130):
    data = body(length)
    standard = base64.b64encode(data).decode()
    rows.append(
        "\t".join(
            [
                data.hex(),
                standard,
                # `Alphabet::UrlSafe` is unpadded, so the fixture records the
                # unpadded form, which is what the crate emits.
                base64.urlsafe_b64encode(data).decode().rstrip("="),
                "1" if standard.endswith("=") else "0",
            ]
        )
    )
# The three-byte tail cases and every single byte value, which is where a
# hand-rolled alphabet table is usually wrong.
for value in range(256):
    data = bytes([value])
    rows.append(
        "\t".join(
            [
                data.hex(),
                base64.b64encode(data).decode(),
                base64.urlsafe_b64encode(data).decode().rstrip("="),
                "1",
            ]
        )
    )
(out_dir / "base64.tsv").write_text("\n".join(rows) + "\n")

print("wrote", len(rows), "base64 cases and", MAX + 257, "digest cases")
