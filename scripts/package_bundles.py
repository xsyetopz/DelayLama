from __future__ import annotations

import argparse
import json
import plistlib
import re
import shutil
import subprocess
from collections.abc import Mapping
from pathlib import Path
from typing import TYPE_CHECKING, TypedDict

if TYPE_CHECKING:
    from artifact_manifest import write_artifact_manifest
    from format_bundles import write_aax_bundle, write_clap_bundle, write_lv2_bundle
else:
    try:
        from .artifact_manifest import write_artifact_manifest
        from .format_bundles import (
            write_aax_bundle,
            write_clap_bundle,
            write_lv2_bundle,
        )
    except ImportError:
        from artifact_manifest import write_artifact_manifest
        from format_bundles import write_aax_bundle, write_clap_bundle, write_lv2_bundle

SUPPORTED_PLATFORMS = frozenset(("macosx", "windows", "linux", "iphoneos"))
PLUGIN_FORMATS = ("vst3", "clap", "au", "auv3", "aax", "lv2")

_AU_V3_HOST_BUNDLE_NAME = "Delay Lama AUv3 Host.app"
_AU_V3_HOST_EXECUTABLE_NAME = "Delay Lama AUv3 Host"
_AU_V3_EXTENSION_POINT = "com.apple.AudioUnit-UI"
_MACOS_SUPPORTED_PLATFORMS = ["MacOSX"]
_MACOS_MINIMUM_SYSTEM_VERSION = "13.0"
_MACOS_BUNDLE_SIGNATURE = "????"

_VERSION_BYTE_MAX = 0xFF
_MAJOR_VERSION_SHIFT = 16
_MINOR_VERSION_SHIFT = 8


class PublisherManifest(TypedDict):
    name: str
    website: str


class AudioUnitManifest(TypedDict):
    manufacturer: str
    subtype: str
    type: str
    auV3ExtensionSuffix: str


class Vst3Manifest(TypedDict):
    name: str
    vendor: str


class ClapManifest(TypedDict):
    id: str
    features: list[str]


class AaxManifest(TypedDict):
    identifier: str
    category: str


class Lv2Manifest(TypedDict):
    uri: str


class IdentityManifest(TypedDict):
    productName: str
    version: str
    publisher: PublisherManifest
    bundleIdentifier: str
    audioUnit: AudioUnitManifest
    vst3: Vst3Manifest
    clap: ClapManifest
    aax: AaxManifest
    lv2: Lv2Manifest


def _required_mapping(
    mapping: dict[str, object], key: str, field: str
) -> dict[str, object]:
    if key not in mapping:
        raise ValueError(f"identity manifest missing {field}")
    value = mapping[key]
    if not isinstance(value, dict):
        raise TypeError(f"identity manifest {field} must be an object")
    return value


def _required_string(mapping: dict[str, object], key: str, field: str) -> str:
    if key not in mapping:
        raise ValueError(f"identity manifest missing {field}")
    value = mapping[key]
    if not isinstance(value, str):
        raise TypeError(f"identity manifest {field} must be a string")
    return value


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("format", choices=PLUGIN_FORMATS)
    parser.add_argument("binary", type=Path)
    parser.add_argument("output_root", type=Path)
    parser.add_argument("identity", type=Path)
    parser.add_argument(
        "--platform", choices=sorted(SUPPORTED_PLATFORMS), default="macosx"
    )
    parser.add_argument("--arch", default="")
    parser.add_argument(
        "--module-info-tool",
        type=Path,
        help="JUCE VST3 manifest helper used to emit factory-derived moduleinfo",
    )
    parser.add_argument(
        "--lv2-manifest-tool",
        type=Path,
        help="JUCE LV2 helper used to emit manifest.ttl, dsp.ttl, and ui.ttl",
    )
    parser.add_argument(
        "--auv3-host-binary",
        type=Path,
        help="xmake-built macOS executable used as the AUv3 containing app",
    )
    return parser.parse_args()


def load_identity(path: Path) -> IdentityManifest:
    with path.open("r", encoding="utf-8") as stream:
        identity = json.load(stream)
    if not isinstance(identity, dict):
        raise TypeError("identity manifest must be an object")
    required = (
        "productName",
        "version",
        "publisher",
        "bundleIdentifier",
        "audioUnit",
        "vst3",
        "clap",
        "aax",
        "lv2",
    )
    missing = [key for key in required if key not in identity]
    if missing:
        raise ValueError(f"identity manifest missing: {', '.join(missing)}")

    publisher = _required_mapping(identity, "publisher", "publisher")
    audio_unit = _required_mapping(identity, "audioUnit", "audioUnit")
    vst3 = _required_mapping(identity, "vst3", "vst3")
    clap = _required_mapping(identity, "clap", "clap")
    clap_features = clap.get("features")
    if not isinstance(clap_features, list) or not clap_features:
        raise TypeError("identity manifest clap.features must be a non-empty array")
    if not all(isinstance(feature, str) and feature for feature in clap_features):
        raise TypeError("identity manifest clap.features must contain strings")
    aax = _required_mapping(identity, "aax", "aax")
    lv2 = _required_mapping(identity, "lv2", "lv2")
    lv2_uri = _required_string(lv2, "uri", "lv2.uri")
    if re.match(r"^(?:https?://|urn:)", lv2_uri) is None:
        raise ValueError(
            "identity manifest lv2.uri must begin with http://, https://, or urn:"
        )
    return {
        "productName": _required_string(identity, "productName", "productName"),
        "version": _required_string(identity, "version", "version"),
        "publisher": {
            "name": _required_string(publisher, "name", "publisher.name"),
            "website": _required_string(publisher, "website", "publisher.website"),
        },
        "bundleIdentifier": _required_string(
            identity, "bundleIdentifier", "bundleIdentifier"
        ),
        "audioUnit": {
            "manufacturer": _required_string(
                audio_unit, "manufacturer", "audioUnit.manufacturer"
            ),
            "subtype": _required_string(audio_unit, "subtype", "audioUnit.subtype"),
            "type": _required_string(audio_unit, "type", "audioUnit.type"),
            "auV3ExtensionSuffix": _required_string(
                audio_unit,
                "auV3ExtensionSuffix",
                "audioUnit.auV3ExtensionSuffix",
            ),
        },
        "vst3": {
            "name": _required_string(vst3, "name", "vst3.name"),
            "vendor": _required_string(vst3, "vendor", "vst3.vendor"),
        },
        "clap": {
            "id": _required_string(clap, "id", "clap.id"),
            "features": clap_features,
        },
        "aax": {
            "identifier": _required_string(aax, "identifier", "aax.identifier"),
            "category": _required_string(aax, "category", "aax.category"),
        },
        "lv2": {"uri": lv2_uri},
    }


def export_prefix(identity: IdentityManifest) -> str:
    product_name = str(identity["productName"])
    prefix = re.sub(r"[^A-Za-z0-9]+", "_", product_name).strip("_")
    if not prefix or not re.match(r"^[A-Za-z_]", prefix):
        raise ValueError(
            "identity manifest productName cannot derive a valid AU export prefix"
        )
    return prefix + "AU"


def audio_component_version(identity: IdentityManifest) -> int:
    version = identity["version"]
    if not isinstance(version, str):
        raise TypeError(
            "identity manifest version must be a numeric major.minor.patch value"
        )
    match = re.fullmatch(r"([0-9]+)\.([0-9]+)\.([0-9]+)", version)
    if match is None:
        raise ValueError(
            "identity manifest version must be a numeric major.minor.patch value"
        )
    major, minor, patch = (int(part) for part in match.groups())
    if any(part > _VERSION_BYTE_MAX for part in (major, minor, patch)):
        raise ValueError(
            "identity manifest version must be a numeric major.minor.patch value"
        )
    return (major << _MAJOR_VERSION_SHIFT) | (minor << _MINOR_VERSION_SHIFT) | patch


def audio_component(identity: IdentityManifest, auv3: bool) -> dict[str, object]:
    publisher = identity["publisher"]
    audio_unit = identity["audioUnit"]
    prefix = export_prefix(identity)
    factory = f"{prefix}FactoryAUv3" if auv3 else f"{prefix}Factory"
    return {
        "name": f"{publisher['name']}: {identity['productName']}",
        "description": identity["productName"],
        "factoryFunction": factory,
        "manufacturer": audio_unit["manufacturer"],
        "type": audio_unit["type"],
        "subtype": audio_unit["subtype"],
        "version": audio_component_version(identity),
        "sandboxSafe": True,
    }


def _write_bundle_plist(metadata_root: Path, info: Mapping[str, object]) -> None:
    metadata_root.mkdir(parents=True, exist_ok=True)
    with (metadata_root / "Info.plist").open("wb") as stream:
        plistlib.dump(dict(info), stream, sort_keys=False)


def _write_pkginfo(metadata_root: Path, package_type: str) -> None:
    # Keep PkgInfo aligned with the plist for LaunchServices inspection.
    (metadata_root / "PkgInfo").write_bytes(
        f"{package_type}{_MACOS_BUNDLE_SIGNATURE}".encode("ascii")
    )


def auv3_host_bundle_identifier(identity: IdentityManifest) -> str:
    return identity["bundleIdentifier"]


def write_apple_bundle(
    identity: IdentityManifest,
    bundle: Path,
    binary: Path,
    auv3: bool,
    platform: str = "macosx",
) -> None:

    if platform not in {"macosx", "iphoneos"}:
        raise ValueError(f"Apple bundles are not supported on {platform!r}")
    if platform == "iphoneos" and not auv3:
        raise ValueError("Audio Unit v2 is not supported on iPhoneOS")

    product = str(identity["productName"])
    if platform == "iphoneos":
        metadata_root = bundle
        executable = bundle / product
    else:
        metadata_root = bundle / "Contents"
        executable = metadata_root / "MacOS" / product
    executable.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(binary, executable)
    bundle_identifier = identity["bundleIdentifier"]
    if auv3:
        bundle_identifier += "." + identity["audioUnit"]["auV3ExtensionSuffix"]
    component = audio_component(identity, auv3)
    info: dict[str, object] = {
        "CFBundleDisplayName": identity["productName"],
        "CFBundleExecutable": identity["productName"],
        "CFBundleIdentifier": bundle_identifier,
        "CFBundleName": identity["productName"],
        "CFBundlePackageType": "XPC!" if auv3 else "BNDL",
        "CFBundleShortVersionString": str(identity["version"]),
        "CFBundleVersion": str(identity["version"]),
    }
    if auv3:
        if platform == "macosx":
            info |= {
                "CFBundleSupportedPlatforms": _MACOS_SUPPORTED_PLATFORMS,
                "LSMinimumSystemVersion": _MACOS_MINIMUM_SYSTEM_VERSION,
                "CFBundleSignature": _MACOS_BUNDLE_SIGNATURE,
            }
        # Keep component identity in one plist authority.
        info["NSExtension"] = {
            "NSExtensionAttributes": {
                "AudioComponents": [component | {"tags": ["Synth"]}]
            },
            "NSExtensionPointIdentifier": _AU_V3_EXTENSION_POINT,
            "NSExtensionPrincipalClass": component["factoryFunction"],
        }
    elif platform == "macosx":
        info["AudioComponents"] = [component]
    _write_bundle_plist(metadata_root, info)
    if auv3 and platform == "macosx":
        _write_pkginfo(metadata_root, "XPC!")
    write_artifact_manifest(
        str(identity["productName"]),
        "auv3" if auv3 else "au",
        platform,
        "",
        bundle,
        metadata_root / "Resources",
        binary,
        executable,
    )


def write_auv3_host_bundle(
    identity: IdentityManifest,
    bundle: Path,
    binary: Path,
    extension_bundle: Path,
) -> None:

    if not binary.is_file():
        raise ValueError(f"missing AUv3 host binary: {binary}")
    contents = bundle / "Contents"
    expected_extension = contents / "PlugIns" / f"{identity['productName']}.appex"
    if extension_bundle != expected_extension:
        raise ValueError(
            "AUv3 extension must be nested at "
            f"{expected_extension}, got {extension_bundle}"
        )
    if not extension_bundle.is_dir():
        raise ValueError(f"missing nested AUv3 extension: {extension_bundle}")

    executable = contents / "MacOS" / _AU_V3_HOST_EXECUTABLE_NAME
    executable.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(binary, executable)
    info: dict[str, object] = {
        "CFBundleDisplayName": _AU_V3_HOST_BUNDLE_NAME.removesuffix(".app"),
        "CFBundleExecutable": _AU_V3_HOST_EXECUTABLE_NAME,
        "CFBundleIdentifier": auv3_host_bundle_identifier(identity),
        "CFBundleName": _AU_V3_HOST_BUNDLE_NAME.removesuffix(".app"),
        "CFBundlePackageType": "APPL",
        "CFBundleShortVersionString": str(identity["version"]),
        "CFBundleVersion": str(identity["version"]),
        "CFBundleSupportedPlatforms": _MACOS_SUPPORTED_PLATFORMS,
        "LSMinimumSystemVersion": _MACOS_MINIMUM_SYSTEM_VERSION,
        "CFBundleSignature": _MACOS_BUNDLE_SIGNATURE,
    }
    _write_bundle_plist(contents, info)
    _write_pkginfo(contents, "APPL")
    write_artifact_manifest(
        str(identity["productName"]),
        "auv3-host",
        "macosx",
        "",
        bundle,
        contents / "Resources",
        binary,
        executable,
    )


def write_vst3_bundle(
    identity: IdentityManifest,
    bundle: Path,
    binary: Path,
    platform: str,
    arch: str,
    module_info_tool: Path,
) -> None:

    contents = bundle / "Contents"
    if platform == "macosx":
        executable = contents / "MacOS" / identity["productName"]
    elif platform == "windows":
        executable = (
            contents / f"{arch or 'x86_64'}-win" / f"{identity['productName']}.vst3"
        )
    elif platform == "linux":
        executable = (
            contents / f"{arch or 'x86_64'}-linux" / f"{identity['productName']}.so"
        )
    else:
        raise ValueError(f"VST3 is not supported on platform {platform!r}")
    executable.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(binary, executable)
    # Emit the same identity plist on portable layouts for standalone inspection.
    bundle_info = {
        "CFBundleDisplayName": identity["productName"],
        "CFBundleExecutable": executable.name,
        "CFBundleIdentifier": identity["bundleIdentifier"],
        "CFBundleName": identity["productName"],
        "CFBundlePackageType": "BNDL",
        "CFBundleShortVersionString": str(identity["version"]),
        "CFBundleVersion": str(identity["version"]),
    }
    _write_bundle_plist(contents, bundle_info)
    if not module_info_tool.is_file():
        raise ValueError(f"missing JUCE VST3 manifest helper: {module_info_tool}")
    manifest = subprocess.run(
        [str(module_info_tool)],
        check=True,
        capture_output=True,
        text=True,
    ).stdout
    if not manifest.strip():
        raise ValueError("JUCE VST3 manifest helper emitted no module information")
    resources = contents / "Resources"
    resources.mkdir(parents=True, exist_ok=True)
    (resources / "moduleinfo.json").write_text(manifest, encoding="utf-8")
    write_artifact_manifest(
        str(identity["productName"]),
        "vst3",
        platform,
        arch,
        bundle,
        contents / "Resources",
        binary,
        executable,
    )


def main() -> None:
    args = parse_args()
    if args.format == "vst3" and args.platform == "iphoneos":
        raise ValueError("VST3 is not supported on iOS")
    if args.format == "clap" and args.platform == "iphoneos":
        raise ValueError("CLAP is not supported on iOS")
    if args.format == "au" and args.platform != "macosx":
        raise ValueError("Audio Unit is supported only on macOS")
    if args.format == "auv3" and args.platform not in {"macosx", "iphoneos"}:
        raise ValueError("Audio Unit v3 is supported only on Apple platforms")
    if args.format == "vst3" and args.module_info_tool is None:
        raise ValueError("VST3 packaging requires --module-info-tool")
    if args.format == "aax" and args.platform not in {"macosx", "windows"}:
        raise ValueError("AAX is supported only on macOS and Windows")
    if args.format == "lv2" and args.platform == "iphoneos":
        raise ValueError("LV2 is not supported on iOS")
    if args.format == "lv2" and args.lv2_manifest_tool is None:
        raise ValueError("LV2 packaging requires --lv2-manifest-tool")
    identity = load_identity(args.identity)
    extension = {
        "vst3": ".vst3",
        "clap": ".clap",
        "au": ".component",
        "auv3": ".appex",
        "aax": ".aaxplugin",
        "lv2": ".lv2",
    }[args.format]
    # Namespace portable outputs to prevent cross-platform bundle collisions.
    bundle_root = (
        args.output_root
        if args.platform == "macosx"
        else args.output_root / args.platform
    )
    format_root = bundle_root / args.format
    bundle = format_root / f"{identity['productName']}{extension}"
    if args.format == "auv3" and args.platform == "macosx":
        # Remove the freestanding extension because PlugInKit installs the nested copy.
        if bundle.exists():
            shutil.rmtree(bundle)
        host_bundle = format_root / _AU_V3_HOST_BUNDLE_NAME
        if host_bundle.exists():
            shutil.rmtree(host_bundle)
        if args.auv3_host_binary is None:
            raise ValueError(
                "macOS AUv3 packaging requires --auv3-host-binary from xmake"
            )
        if not args.auv3_host_binary.is_file():
            raise ValueError(f"missing AUv3 host binary: {args.auv3_host_binary}")
        extension_bundle = (
            host_bundle
            / "Contents"
            / "PlugIns"
            / f"{identity['productName']}{extension}"
        )
        write_apple_bundle(
            identity,
            extension_bundle,
            args.binary,
            True,
            args.platform,
        )
        write_auv3_host_bundle(
            identity,
            host_bundle,
            args.auv3_host_binary,
            extension_bundle,
        )
        print(f"assembled {args.format} bundle: {host_bundle}")
        return
    if bundle.exists():
        # Remove stale output before switching between file and directory layouts.
        if bundle.is_dir():
            shutil.rmtree(bundle)
        else:
            bundle.unlink()
    if args.format == "vst3":
        write_vst3_bundle(
            identity,
            bundle,
            args.binary,
            args.platform,
            args.arch,
            args.module_info_tool,
        )
    elif args.format == "clap":
        write_clap_bundle(
            identity,
            bundle,
            args.binary,
            args.platform,
            args.arch,
        )
    elif args.format in {"au", "auv3"}:
        write_apple_bundle(
            identity,
            bundle,
            args.binary,
            args.format == "auv3",
            args.platform,
        )
    elif args.format == "aax":
        write_aax_bundle(identity, bundle, args.binary, args.platform, args.arch)
    else:
        if args.lv2_manifest_tool is None:
            raise ValueError("LV2 packaging requires --lv2-manifest-tool")
        write_lv2_bundle(
            identity,
            bundle,
            args.binary,
            args.platform,
            args.arch,
            args.lv2_manifest_tool,
        )
    print(f"assembled {args.format} bundle: {bundle}")


if __name__ == "__main__":
    main()
