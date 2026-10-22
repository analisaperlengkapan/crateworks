"""Generate the signed-PDF corpus used to test `pdfa` and the proposed `pades`.

The oracle is `pyhanko`, the same package that produced the fixtures committed
under `crates/pdfa/tests/fixtures/`. `reportlab` draws the page, `pyhanko` signs
it. Neither is ever a dependency of a crate here; both are development tools.

    pip install pyhanko reportlab cryptography
    python3 research/tools/gen_pades_corpus.py crates/pdfa/tests/fixtures

It writes, for each level, a signed document plus a `.json` label recording what
`pyhanko` reports about it. The labels are the fixture: a test asserts on the
labels, never on prose, so a wording change cannot fail a test and a behaviour
change always does.

The negative cases matter more than the positive ones. `flip_byte` and
`append_byte` produce a document that a careless reader accepts, and they are the
cases that make an integrity check worth having.
"""

from __future__ import annotations

import base64
import json
import pathlib
import sys

from cryptography import x509
from cryptography.hazmat.primitives import hashes, serialization
from cryptography.hazmat.primitives.asymmetric import rsa
from cryptography.x509.oid import NameOID
from pyhanko.pdf_utils.incremental_writer import IncrementalPdfFileWriter
from pyhanko.pdf_utils.reader import PdfFileReader
from pyhanko.sign import fields, signers
from pyhanko.sign.validation import validate_pdf_signature
from pyhanko_certvalidator import ValidationContext
from reportlab.pdfgen import canvas

import datetime

from asn1crypto import x509 as a_x509


def make_identity(directory: pathlib.Path):
    """A self-signed RSA key and certificate, valid for twenty years."""
    key = rsa.generate_private_key(public_exponent=65537, key_size=2048)
    name = x509.Name([x509.NameAttribute(NameOID.COMMON_NAME, "PAdES Corpus")])
    cert = (
        x509.CertificateBuilder()
        .subject_name(name)
        .issuer_name(name)
        .public_key(key.public_key())
        .serial_number(x509.random_serial_number())
        .not_valid_before(datetime.datetime(2020, 1, 1))
        .not_valid_after(datetime.datetime(2040, 1, 1))
        .add_extension(
            # `content_commitment` is the nonRepudiation bit, and pyhanko refuses
            # to sign without it.
            x509.KeyUsage(
                digital_signature=True,
                content_commitment=True,
                key_encipherment=False,
                data_encipherment=False,
                key_agreement=False,
                key_cert_sign=True,
                crl_sign=True,
                encipher_only=False,
                decipher_only=False,
            ),
            critical=True,
        )
        .add_extension(x509.BasicConstraints(ca=True, path_length=None), critical=True)
        .sign(key, hashes.SHA256())
    )
    key_path = directory / "key.pem"
    cert_path = directory / "cert.pem"
    key_path.write_bytes(
        key.private_bytes(
            serialization.Encoding.PEM,
            serialization.PrivateFormat.PKCS8,
            serialization.NoEncryption(),
        )
    )
    cert_path.write_bytes(cert.public_bytes(serialization.Encoding.PEM))
    der = base64.b64decode(
        "".join(line for line in cert_path.read_text().splitlines() if "CERTIFICATE" not in line)
    )
    return key_path, cert_path, a_x509.Certificate.load(der)


def make_page(path: pathlib.Path) -> None:
    """A one-page PDF with a single word on it."""
    pdf = canvas.Canvas(str(path), pagesize=(200, 200))
    pdf.drawString(20, 100, "corpus")
    pdf.showPage()
    pdf.save()


def sign(blank: pathlib.Path, out: pathlib.Path, key, cert, root, *, long_term: bool) -> None:
    """Sign `blank` into `out`, at B-B or, with `long_term`, at B-LT."""
    signer = signers.SimpleSigner.load(str(key), str(cert))
    context = ValidationContext(trust_roots=[root], allow_fetching=False)
    metadata = signers.PdfSignatureMetadata(
        field_name="Sig1",
        subfilter=fields.SigSeedSubFilter.PADES,
        md_algorithm="sha256",
        # `embed_validation_info` is frozen at construction, so it is passed here
        # rather than assigned after. With it, pyhanko writes the DSS.
        embed_validation_info=long_term,
        validation_context=context if long_term else None,
    )
    writer = IncrementalPdfFileWriter(open(blank, "rb"))
    signed = signers.sign_pdf(writer, metadata, signer=signer)
    out.write_bytes(signed.getbuffer())


def label(path: pathlib.Path, root) -> dict:
    """What pyhanko reports about `path`, as the expected answer for a test.

    A corrupted file can make pyhanko raise rather than report, so a failure is
    recorded as a label instead of stopping the run. That is itself an answer:
    the oracle could not read it, so this crate must not accept it either.
    """
    context = ValidationContext(trust_roots=[root], allow_fetching=False)
    try:
        reader = PdfFileReader(open(path, "rb"))
        signatures = []
        for signature in reader.embedded_signatures:
            status = validate_pdf_signature(signature, context)
            signatures.append(
                {
                    "field": signature.field_name,
                    "subfilter": str(signature.sig_object.get("/SubFilter")),
                    "digest": signature.md_algorithm,
                    "intact": bool(status.intact),
                    "valid": bool(status.valid),
                    "trusted": bool(status.trusted),
                    "coverage": str(status.coverage).rsplit(".", 1)[-1],
                }
            )
    except Exception as exc:  # noqa: BLE001 - the failure is the label
        return {"oracle_error": f"{type(exc).__name__}", "signatures": []}
    return {"signatures": signatures}


def flip_byte(data: bytes) -> bytes:
    """Change one byte inside a content stream of the first revision.

    The byte is chosen inside a `stream ... endstream` pair, before the
    signature was appended, so it is inside the signed range but outside the
    cross-reference table and outside the CMS. Flipping a byte near the end of
    the revision instead lands in the xref table, which makes the file
    unreadable rather than tampered, and an unreadable file tests nothing about
    the integrity check.
    """
    body = bytearray(data)
    start = body.find(b"stream")
    assert start != -1, "the first revision should hold a stream"
    # Step past the `stream` keyword and its end-of-line, into the payload.
    at = start + len(b"stream") + 2
    assert body[at : at + 2] not in (b"en", b"%%"), "expected a stream payload here"
    body[at] ^= 0x01
    return bytes(body)


def append_byte(data: bytes) -> bytes:
    """Add a byte after the signed range, so the coverage is no longer the file."""
    return data + b"\n"


def main(argv: list[str]) -> int:
    if len(argv) != 2:
        print(__doc__)
        return 2
    out = pathlib.Path(argv[1])
    out.mkdir(parents=True, exist_ok=True)

    key, cert, root = make_identity(out)
    blank = out / "blank.pdf"
    make_page(blank)

    cases = {"signed_bb.pdf": False, "signed_lt.pdf": True}
    for name, long_term in cases.items():
        target = out / name
        sign(blank, target, key, cert, root, long_term=long_term)
        (out / f"{name}.json").write_text(json.dumps(label(target, root), indent=2) + "\n")
        print(f"wrote {name} ({target.stat().st_size} bytes)")

    # The negative cases: a reader that only checks the structure accepts these.
    for name, mutate in [("tampered.pdf", flip_byte), ("appended.pdf", append_byte)]:
        source = (out / "signed_bb.pdf").read_bytes()
        target = out / name
        target.write_bytes(mutate(source))
        (out / f"{name}.json").write_text(json.dumps(label(target, root), indent=2) + "\n")
        print(f"wrote {name} ({target.stat().st_size} bytes)")

    # The key is regenerated on every run, so it is not committed.
    for generated in (key, cert):
        generated.unlink()
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
