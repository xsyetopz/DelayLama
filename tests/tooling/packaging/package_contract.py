from __future__ import annotations

import json
import plistlib
import tempfile
import unittest
from pathlib import Path

from scripts.artifact_manifest import verify_artifact_manifest
from scripts.package_bundles import (
    IdentityManifest,
    write_aax_bundle,
    write_apple_bundle,
    write_clap_bundle,
    write_lv2_bundle,
)

_EXECUTABLE_PERMISSIONS = 0o755


def _identity() -> IdentityManifest:
    return {
        "productName": "Delay Lama",
        "version": "1.0.0",
        "publisher": {"name": "xsyetopz", "website": "https://xsyetopz.com"},
        "bundleIdentifier": "com.xsyetopz.delaylama",
        "audioUnit": {
            "manufacturer": "xsyT",
            "subtype": "DLma",
            "type": "aumu",
            "auV3ExtensionSuffix": "delaylamaAUv3",
        },
        "vst3": {"name": "Delay Lama", "vendor": "xsyetopz"},
        "clap": {
            "id": "com.xsyetopz.delaylama",
            "features": ["instrument", "synthesizer"],
        },
        "aax": {
            "identifier": "com.xsyetopz.delaylama",
            "category": "SWGenerators",
        },
        "lv2": {"uri": "https://xsyetopz.com/plugins/delay-lama"},
    }


class BundleArtifactContractTests(unittest.TestCase):
    def test_apple_bundle_records_and_verifies_payload_digest(self) -> None:
        identity = _identity()
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = Path(temporary_directory)
            source = root / "DelayLama_AUcomponent"
            source.write_bytes(b"validated Delay Lama payload\n")
            bundle = root / "Delay Lama.component"

            write_apple_bundle(identity, bundle, source, False, "macosx")

            executable = bundle / "Contents" / "MacOS" / "Delay Lama"
            manifest_path = bundle / "Contents" / "Resources" / "artifact-manifest.json"
            manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
            self.assertEqual(manifest["format"], "au")
            self.assertEqual(manifest["payload"]["path"], "Contents/MacOS/Delay Lama")
            with (bundle / "Contents" / "Info.plist").open("rb") as stream:
                metadata = plistlib.load(stream)
            self.assertEqual(
                metadata["AudioComponents"][0]["factoryFunction"],
                "Delay_LamaAUFactory",
            )
            verify_artifact_manifest(
                "Delay Lama", bundle, executable, "au", "macosx", False
            )

            executable.write_bytes(b"stale payload\n")
            with self.assertRaises(AssertionError):
                verify_artifact_manifest(
                    "Delay Lama", bundle, executable, "au", "macosx", False
                )
            verify_artifact_manifest(
                "Delay Lama",
                bundle,
                executable,
                "au",
                "macosx",
                False,
                allow_signed_payload=True,
            )

    def test_clap_bundle_records_identity_and_native_payload(self) -> None:
        identity = _identity()
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = Path(temporary_directory)
            source = root / "DelayLama_CLAPclap"
            source.write_bytes(b"CLAP module\n")
            bundle = root / "Delay Lama.clap"
            write_clap_bundle(identity, bundle, source, "macosx", "arm64")
            executable = bundle / "Contents" / "MacOS" / "Delay Lama"
            with (bundle / "Contents" / "Info.plist").open("rb") as stream:
                metadata = plistlib.load(stream)
            self.assertEqual(metadata["CFBundleIdentifier"], identity["clap"]["id"])
            verify_artifact_manifest(
                "Delay Lama", bundle, executable, "clap", "macosx", False
            )

            portable = root / "linux" / "clap" / "Delay Lama.clap"
            write_clap_bundle(identity, portable, source, "linux", "x86_64")
            verify_artifact_manifest(
                "Delay Lama", portable.parent, portable, "clap", "linux", False
            )
            portable.write_bytes(b"tampered CLAP module\n")
            with self.assertRaises(AssertionError):
                verify_artifact_manifest(
                    "Delay Lama", portable.parent, portable, "clap", "linux", False
                )

    def test_aax_bundle_records_identity_and_native_payload(self) -> None:
        identity = _identity()
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = Path(temporary_directory)
            source = root / "DelayLama_AAXaaxplugin"
            source.write_bytes(b"AAX module\n")
            bundle = root / "Delay Lama.aaxplugin"
            write_aax_bundle(identity, bundle, source, "macosx", "arm64")
            executable = bundle / "Contents" / "MacOS" / "Delay Lama"
            with (bundle / "Contents" / "Info.plist").open("rb") as stream:
                metadata = plistlib.load(stream)
            self.assertEqual(metadata["CFBundleIdentifier"], "com.xsyetopz.delaylama")
            verify_artifact_manifest(
                "Delay Lama", bundle, executable, "aax", "macosx", False
            )

    def test_lv2_bundle_uses_helper_turtle_and_configured_uri(self) -> None:
        identity = _identity()
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = Path(temporary_directory)
            source = root / "DelayLama_LV2.so"
            source.write_bytes(b"LV2 module\n")
            helper = root / "lv2-helper"
            helper.write_text(
                "#!/usr/bin/env python3\n"
                "from pathlib import Path\n"
                "import sys\n"
                "root = Path(sys.argv[1]).parent\n"
                "(root / 'manifest.ttl').write_text("
                "'<https://xsyetopz.com/plugins/delay-lama>\\n')\n"
                "(root / 'dsp.ttl').write_text('dsp\\n')\n"
                "(root / 'ui.ttl').write_text('ui\\n')\n",
                encoding="utf-8",
            )
            helper.chmod(_EXECUTABLE_PERMISSIONS)
            bundle = root / "Delay Lama.lv2"
            write_lv2_bundle(identity, bundle, source, "macosx", "arm64", helper)
            self.assertIn(
                "<https://xsyetopz.com/plugins/delay-lama>",
                (bundle / "manifest.ttl").read_text(encoding="utf-8"),
            )
            artifact = json.loads(
                (bundle / "artifact-manifest.json").read_text(encoding="utf-8")
            )
            self.assertEqual(artifact["format"], "lv2")
            self.assertEqual(artifact["payload"]["path"], "Delay Lama.so")


if __name__ == "__main__":
    unittest.main()
