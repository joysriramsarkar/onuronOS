#!/usr/bin/env python3
"""
build/mkvbmeta.py — Cryptographic Verified Boot Descriptor Tool (Ed25519).

Generates and verifies cryptographic vbmeta images containing:
- DM-verity root SHA-256 hash
- Cryptographic Ed25519 signature of partition metadata & root hash
- Ed25519 public key for chain-of-trust verification

Fail-closed: Rejects missing images, corrupt headers, or tampered signatures.
"""

import argparse
import hashlib
import os
import sys

try:
    from cryptography.hazmat.primitives.asymmetric import ed25519
    from cryptography.hazmat.primitives import serialization
except ImportError:
    ed25519 = None


def compute_sha256(filepath: str) -> str:
    """Computes SHA-256 hash of a file in streaming chunks."""
    h = hashlib.sha256()
    with open(filepath, "rb") as f:
        while chunk := f.read(65536):
            h.update(chunk)
    return h.hexdigest()


def get_or_create_key(key_path: str = None):
    """Loads Ed25519 private key from path or generates an ephemeral key."""
    if key_path and os.path.isfile(key_path):
        with open(key_path, "rb") as f:
            data = f.read()
            if len(data) == 32:
                return ed25519.Ed25519PrivateKey.from_private_bytes(data)
            return serialization.load_pem_private_key(data, password=None)
    
    # Generate new key
    priv = ed25519.Ed25519PrivateKey.generate()
    if key_path:
        os.makedirs(os.path.dirname(os.path.abspath(key_path)), exist_ok=True)
        with open(key_path, "wb") as f:
            f.write(priv.private_bytes(
                encoding=serialization.Encoding.Raw,
                format=serialization.PrivateFormat.Raw,
                encryption_algorithm=serialization.NoEncryption()
            ))
    return priv


def create_vbmeta(image_path: str, output_path: str, key_path: str = None, partition_name: str = "system") -> bool:
    """Creates a real signed vbmeta descriptor image."""
    if not os.path.isfile(image_path):
        print(f"[ERROR] Image file does not exist: {image_path}", file=sys.stderr)
        return False

    if ed25519 is None:
        print("[ERROR] 'cryptography' library is required for Verified Boot Ed25519 signing", file=sys.stderr)
        return False

    image_size = os.path.getsize(image_path)
    root_hash = compute_sha256(image_path)

    priv_key = get_or_create_key(key_path)
    pub_key = priv_key.public_key()
    pub_bytes = pub_key.public_bytes(
        encoding=serialization.Encoding.Raw,
        format=serialization.PublicFormat.Raw
    )

    # Payload to sign: partition:size:root_hash
    signed_payload = f"ONURON_VBMETA_V1:{partition_name}:{image_size}:{root_hash}".encode("utf-8")
    signature = priv_key.sign(signed_payload)

    os.makedirs(os.path.dirname(os.path.abspath(output_path)), exist_ok=True)
    with open(output_path, "w", encoding="utf-8") as f:
        f.write("# OnuronOS Verified Boot Descriptor (VBMeta V1)\n")
        f.write(f"FORMAT_VERSION=1\n")
        f.write(f"PARTITION_NAME={partition_name}\n")
        f.write(f"IMAGE_SIZE={image_size}\n")
        f.write(f"ROOT_HASH={root_hash}\n")
        f.write(f"PUBLIC_KEY={pub_bytes.hex()}\n")
        f.write(f"SIGNATURE={signature.hex()}\n")

    print(f"[OK] vbmeta generated: {output_path} (Partition: {partition_name}, Root Hash: {root_hash[:16]}...)")
    return True


def verify_vbmeta(image_path: str, vbmeta_path: str, expected_pubkey_hex: str = None) -> bool:
    """Verifies image integrity and Ed25519 signature from vbmeta."""
    if not os.path.isfile(image_path):
        print(f"[ERROR] Image missing: {image_path}", file=sys.stderr)
        return False
    if not os.path.isfile(vbmeta_path):
        print(f"[ERROR] vbmeta missing: {vbmeta_path}", file=sys.stderr)
        return False

    fields = {}
    with open(vbmeta_path, "r", encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if "=" in line and not line.startswith("#"):
                k, v = line.split("=", 1)
                fields[k] = v

    for req in ("PARTITION_NAME", "IMAGE_SIZE", "ROOT_HASH", "PUBLIC_KEY", "SIGNATURE"):
        if req not in fields:
            print(f"[ERROR] Malformed vbmeta missing field: {req}", file=sys.stderr)
            return False

    pub_hex = fields["PUBLIC_KEY"]
    if expected_pubkey_hex and pub_hex != expected_pubkey_hex:
        print("[ERROR] Public key mismatch (untrusted signing authority)", file=sys.stderr)
        return False

    actual_hash = compute_sha256(image_path)
    if actual_hash != fields["ROOT_HASH"]:
        print(f"[ERROR] Image tampered! Expected hash {fields['ROOT_HASH']}, got {actual_hash}", file=sys.stderr)
        return False

    actual_size = os.path.getsize(image_path)
    if str(actual_size) != fields["IMAGE_SIZE"]:
        print(f"[ERROR] Image size mismatch! Expected {fields['IMAGE_SIZE']}, got {actual_size}", file=sys.stderr)
        return False

    try:
        pub_bytes = bytes.fromhex(pub_hex)
        sig_bytes = bytes.fromhex(fields["SIGNATURE"])
        pub_key = ed25519.Ed25519PublicKey.from_public_bytes(pub_bytes)
        signed_payload = f"ONURON_VBMETA_V1:{fields['PARTITION_NAME']}:{fields['IMAGE_SIZE']}:{fields['ROOT_HASH']}".encode("utf-8")
        pub_key.verify(sig_bytes, signed_payload)
        print(f"[OK] Verified Boot signature check PASSED for {image_path}")
        return True
    except Exception as e:
        print(f"[ERROR] Ed25519 signature verification FAILED: {e}", file=sys.stderr)
        return False


def main():
    parser = argparse.ArgumentParser(description="OnuronOS Verified Boot VBMeta Tool")
    subparsers = parser.add_subparsers(dest="action", required=True)

    create_p = subparsers.add_parser("create")
    create_p.add_argument("--image", required=True, help="Path to partition image")
    create_p.add_argument("-o", "--output", required=True, help="Output vbmeta path")
    create_p.add_argument("--key", help="Path to Ed25519 private key")
    create_p.add_argument("--partition", default="system", help="Partition name")

    verify_p = subparsers.add_parser("verify")
    verify_p.add_argument("--image", required=True, help="Path to partition image")
    verify_p.add_argument("--vbmeta", required=True, help="Path to vbmeta file")
    verify_p.add_argument("--pubkey", help="Expected public key hex")

    args = parser.parse_args()
    if args.action == "create":
        ok = create_vbmeta(args.image, args.output, args.key, args.partition)
        sys.exit(0 if ok else 1)
    elif args.action == "verify":
        ok = verify_vbmeta(args.image, args.vbmeta, args.pubkey)
        sys.exit(0 if ok else 1)


if __name__ == "__main__":
    main()
