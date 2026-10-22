#!/usr/bin/env python3
"""Regenerates the RSA test vectors in crates/pades/src/rsa.rs.

Run this only when the vector has to change, and paste the output into the
`vector` function. The modulus is 1024 bits so the literal stays readable; a
2048-bit vector would be four times as long for the same coverage.

    pip install cryptography
    python3 research/tools/gen_rsa_vectors.py
"""

from cryptography.hazmat.primitives import hashes
from cryptography.hazmat.primitives.asymmetric import padding, rsa

MESSAGE = b"abc"


def wrap(data: bytes, indent: int = 12) -> str:
    text = data.hex()
    pieces = [text[index : index + 64] for index in range(0, len(text), 64)]
    return ("\\\n" + " " * indent).join(pieces)


def main() -> None:
    key = rsa.generate_private_key(public_exponent=65537, key_size=1024)
    numbers = key.public_key().public_numbers()
    modulus = numbers.n.to_bytes(128, "big")
    pkcs1v15 = key.sign(MESSAGE, padding.PKCS1v15(), hashes.SHA256())
    pss = key.sign(
        MESSAGE,
        padding.PSS(mgf=padding.MGF1(hashes.SHA256()), salt_length=32),
        hashes.SHA256(),
    )
    print("let modulus = hex(")
    print(f'    "{wrap(modulus)}",')
    print(");")
    print("let pkcs1v15 = hex(")
    print(f'    "{wrap(pkcs1v15)}",')
    print(");")
    print("let pss = hex(")
    print(f'    "{wrap(pss)}",')
    print(");")


if __name__ == "__main__":
    main()
