from __future__ import annotations

import hashlib
import json
from pathlib import Path
from typing import Any, cast

ARTIFACT_MANIFEST_NAME = "artifact-manifest.json"
ARTIFACT_MANIFEST_SCHEMA = 1
_HASH_CHUNK_BYTES = 1024 * 1024
_MANIFEST_INDENT = 2


def _sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        while chunk := stream.read(_HASH_CHUNK_BYTES):
            digest.update(chunk)
    return digest.hexdigest()


def write_artifact_manifest(
    product_name: str,
    format_name: str,
    platform: str,
    arch: str,
    bundle: Path,
    metadata_root: Path,
    source_binary: Path,
    payload: Path,
) -> None:

    if not source_binary.is_file():
        raise ValueError(f"missing source artifact: {source_binary}")
    if not payload.is_file():
        raise ValueError(f"missing packaged payload: {payload}")
    try:
        relative_payload = payload.relative_to(bundle).as_posix()
    except ValueError as error:
        raise ValueError(f"packaged payload is outside bundle: {payload}") from error

    source_sha256 = _sha256_file(source_binary)
    payload_sha256 = _sha256_file(payload)
    if source_sha256 != payload_sha256:
        raise ValueError(
            "packaged payload differs from source artifact: "
            f"{source_binary} != {payload}"
        )

    manifest = {
        "arch": arch,
        "format": format_name,
        "input": {"name": source_binary.name, "sha256": source_sha256},
        "payload": {"path": relative_payload, "sha256": payload_sha256},
        "platform": platform,
        "productName": product_name,
        "schema": ARTIFACT_MANIFEST_SCHEMA,
    }
    metadata_root.mkdir(parents=True, exist_ok=True)
    (metadata_root / ARTIFACT_MANIFEST_NAME).write_text(
        json.dumps(manifest, indent=_MANIFEST_INDENT, sort_keys=True) + "\n",
        encoding="utf-8",
    )


def _require(condition: bool, message: str) -> None:
    if not condition:
        raise AssertionError(message)


def verify_artifact_manifest(
    product_name: str,
    bundle: Path,
    executable: Path,
    format_name: str,
    platform: str,
    iphoneos: bool,
    allow_signed_payload: bool = False,
) -> None:

    if iphoneos:
        manifest_path = bundle / "Resources" / ARTIFACT_MANIFEST_NAME
    elif format_name == "clap" and platform in {"linux", "windows"}:
        manifest_path = bundle / ARTIFACT_MANIFEST_NAME
    else:
        manifest_path = bundle / "Contents" / "Resources" / ARTIFACT_MANIFEST_NAME
    _require(manifest_path.is_file(), f"missing artifact manifest: {manifest_path}")
    with manifest_path.open("r", encoding="utf-8") as handle:
        raw_manifest: object = json.load(handle)
    _require(
        isinstance(raw_manifest, dict), f"invalid artifact manifest: {manifest_path}"
    )
    manifest = cast(dict[str, Any], raw_manifest)
    context = f"artifact {bundle}"
    _require(
        manifest.get("schema") == ARTIFACT_MANIFEST_SCHEMA,
        f"{context}: wrong schema",
    )
    _require(manifest.get("productName") == product_name, f"{context}: wrong product")
    _require(manifest.get("format") == format_name, f"{context}: wrong format")
    _require(manifest.get("platform") == platform, f"{context}: wrong platform")
    input_record = manifest.get("input", {})
    payload_record = manifest.get("payload", {})
    _require(isinstance(input_record, dict), f"{context}: malformed input record")
    _require(isinstance(payload_record, dict), f"{context}: malformed payload record")
    _require(
        isinstance(input_record.get("name"), str) and bool(input_record["name"]),
        f"{context}: artifact input record is missing its name",
    )
    try:
        relative_payload = executable.relative_to(bundle).as_posix()
    except ValueError as error:
        raise AssertionError(f"{context}: executable is outside bundle") from error
    _require(
        payload_record.get("path") == relative_payload,
        f"{context}: payload path does not identify the executable",
    )
    input_sha256 = input_record.get("sha256")
    payload_sha256 = payload_record.get("sha256")
    _require(
        isinstance(input_sha256, str) and isinstance(payload_sha256, str),
        f"{context}: artifact records must contain SHA-256 strings",
    )
    _require(
        input_sha256 == payload_sha256,
        f"{context}: source and packaged SHA-256 values differ",
    )
    if not allow_signed_payload:
        _require(
            _sha256_file(executable) == payload_sha256,
            f"{context}: packaged executable differs from its build artifact",
        )
