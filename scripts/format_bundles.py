from __future__ import annotations

import plistlib
import shutil
import subprocess
import sys
from collections.abc import Mapping
from pathlib import Path
from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:
    from artifact_manifest import write_artifact_manifest
else:
    try:
        from .artifact_manifest import write_artifact_manifest
    except ImportError:
        from artifact_manifest import write_artifact_manifest


def _write_bundle_plist(metadata_root: Path, info: Mapping[str, object]) -> None:
    metadata_root.mkdir(parents=True, exist_ok=True)
    with (metadata_root / "Info.plist").open("wb") as stream:
        plistlib.dump(dict(info), stream, sort_keys=False)


def write_clap_bundle(
    identity: Mapping[str, Any],
    bundle: Path,
    binary: Path,
    platform: str,
    arch: str,
) -> None:

    product = str(identity["productName"])
    if platform == "macosx":
        contents = bundle / "Contents"
        executable = contents / "MacOS" / product
        executable.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(binary, executable)
        _write_bundle_plist(
            contents,
            {
                "CFBundleDevelopmentRegion": "English",
                "CFBundleDisplayName": product,
                "CFBundleExecutable": product,
                "CFBundleIdentifier": identity["clap"]["id"],
                "CFBundleName": product,
                "CFBundlePackageType": "BNDL",
                "CFBundleShortVersionString": identity["version"],
                "CFBundleVersion": identity["version"],
            },
        )
        write_artifact_manifest(
            product,
            "clap",
            platform,
            arch,
            bundle,
            contents / "Resources",
            binary,
            executable,
        )
        return

    if platform not in {"windows", "linux"}:
        raise ValueError(f"CLAP is not supported on platform {platform!r}")
    bundle.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(binary, bundle)
    write_artifact_manifest(
        product,
        "clap",
        platform,
        arch,
        bundle.parent,
        bundle.parent,
        binary,
        bundle,
    )


def write_aax_bundle(
    identity: Mapping[str, Any],
    bundle: Path,
    binary: Path,
    platform: str,
    arch: str,
) -> None:

    contents = bundle / "Contents"
    if platform == "macosx":
        executable = contents / "MacOS" / identity["productName"]
        metadata_root = contents
        info: dict[str, object] = {
            "CFBundleDisplayName": identity["productName"],
            "CFBundleExecutable": identity["productName"],
            "CFBundleIdentifier": identity["aax"]["identifier"],
            "CFBundleName": identity["productName"],
            "CFBundlePackageType": "BNDL",
            "CFBundleShortVersionString": identity["version"],
            "CFBundleVersion": identity["version"],
        }
        _write_bundle_plist(metadata_root, info)
    elif platform == "windows":
        executable = (
            contents
            / ("Win32" if arch in {"x86", "i386"} else "x64")
            / f"{identity['productName']}.aaxplugin"
        )
        metadata_root = contents
    else:
        raise ValueError(f"AAX is not supported on platform {platform!r}")
    executable.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(binary, executable)
    write_artifact_manifest(
        identity["productName"],
        "aax",
        platform,
        arch,
        bundle,
        metadata_root / "Resources",
        binary,
        executable,
    )


def write_lv2_bundle(
    identity: Mapping[str, Any],
    bundle: Path,
    binary: Path,
    platform: str,
    arch: str,
    manifest_tool: Path,
) -> None:

    suffix = ".dll" if platform == "windows" else ".so"
    executable = bundle / f"{identity['productName']}{suffix}"
    executable.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(binary, executable)
    if not manifest_tool.is_file():
        raise ValueError(f"missing JUCE LV2 manifest helper: {manifest_tool}")
    command = [str(manifest_tool), str(executable)]
    if manifest_tool.suffix.lower() == ".py":
        command.insert(0, sys.executable)
    subprocess.run(command, check=True)
    for turtle_name in ("manifest.ttl", "dsp.ttl", "ui.ttl"):
        turtle = bundle / turtle_name
        if not turtle.is_file() or not turtle.read_text(encoding="utf-8").strip():
            raise ValueError(f"JUCE LV2 helper did not emit {turtle_name}")
    manifest_text = (bundle / "manifest.ttl").read_text(encoding="utf-8")
    if f"<{identity['lv2']['uri']}>" not in manifest_text:
        raise ValueError("JUCE LV2 manifest does not contain the configured URI")
    write_artifact_manifest(
        identity["productName"],
        "lv2",
        platform,
        arch,
        bundle,
        bundle,
        binary,
        executable,
    )
