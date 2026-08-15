#!/usr/bin/env python3
"""Build Delay Lama AUv3 with fixed-size macOS host negotiation.

cargo-truce 6.3.0 advertises every host view configuration and makes the
controller view width/height sizable even when the Rust editor opts out of
resizing. Logic consequently expands the fixed 360x510 editor to its entire
2000x1752 plug-in pane. This bounded source patch is restored after the build.
"""

from __future__ import annotations

import shutil
import subprocess
from pathlib import Path

TEMPLATE_RELATIVE = Path("cargo-truce-6.3.0/templates/au3/AudioUnitFactory.swift")
OLD_CONFIG = """override func supportedViewConfigurations(
        _ availableViewConfigurations: [AUAudioUnitViewConfiguration]
    ) -> IndexSet {
        IndexSet(integersIn: availableViewConfigurations.indices)
    }"""
NEW_CONFIG = """override func supportedViewConfigurations(
        _ availableViewConfigurations: [AUAudioUnitViewConfiguration]
    ) -> IndexSet {
        // This editor has one intrinsic size, not a host-selectable layout.
        // Returning any proposed configuration opts Logic into its resizable
        // plug-in pane, whose outer window remains larger than the editor.
        IndexSet()
    }"""
OLD_SELECT = (
    "override func select(_ viewConfiguration: AUAudioUnitViewConfiguration) {}"
)
NEW_SELECT = """override func select(_ viewConfiguration: AUAudioUnitViewConfiguration) {
        // No host configuration is supported for this fixed-size editor.
    }"""
OLD_MASK = "v.autoresizingMask = [.width, .height]"
NEW_MASK = "v.autoresizingMask = [] // fixed-size Delay Lama editor"


OLD_WILL_APPEAR = r"""override func viewWillAppear() {
        super.viewWillAppear()
        logger.info("viewWillAppear: view.frame=\(self.view.frame.width)x\(self.view.frame.height)")
        setupGUIIfReady()
    }"""
NEW_WILL_APPEAR = r"""override func viewWillAppear() {
        super.viewWillAppear()
        let fixedEditorSize = NSSize(width: 360.0, height: 510.0)
        preferredContentSize = fixedEditorSize
        view.setFrameSize(fixedEditorSize)
        logger.info("viewWillAppear fixed: \(self.view.frame.width)x\(self.view.frame.height)")
        setupGUIIfReady()
    }"""


def find_template() -> Path:
    cargo_home = Path.home() / ".cargo" / "registry" / "src"
    matches = list(cargo_home.glob(f"*/{TEMPLATE_RELATIVE}"))
    if len(matches) != 1:
        raise RuntimeError(
            f"expected one cargo-truce 6.3.0 AUv3 template, found {matches}"
        )
    return matches[0]


def patched(source: str) -> str:
    if (
        source.count(OLD_CONFIG) != 1
        or source.count(OLD_SELECT) != 1
        or source.count(OLD_MASK) != 1
        or source.count(OLD_WILL_APPEAR) != 1
    ):
        raise RuntimeError(
            "cargo-truce AUv3 template no longer matches the verified 6.3.0 source"
        )
    return (
        source.replace(OLD_CONFIG, NEW_CONFIG)
        .replace(OLD_SELECT, NEW_SELECT)
        .replace(OLD_MASK, NEW_MASK)
        .replace(OLD_WILL_APPEAR, NEW_WILL_APPEAR)
    )


def main() -> None:
    template = find_template()
    patched_tool = Path("/tmp/openlama-cargo-truce-fixed-auv3")
    shutil.rmtree(patched_tool, ignore_errors=True)
    shutil.copytree(template.parent.parent.parent, patched_tool)
    copied_template = patched_tool / "templates/au3/AudioUnitFactory.swift"
    copied_template.write_text(patched(copied_template.read_text()))

    # AudioUnitFactory.swift is embedded into cargo-truce at compile time.
    # Running the installed binary after changing the registry file would still
    # emit its old source, so compile and run this isolated patched tool copy.
    shutil.rmtree(Path("target/tmp/au-v3"), ignore_errors=True)
    subprocess.run(
        [
            "cargo",
            "run",
            "--release",
            "--manifest-path",
            str(patched_tool / "Cargo.toml"),
            "--",
            "build",
            "--au3",
            "-p",
            "delaylama-truce-plugin",
        ],
        check=True,
    )


if __name__ == "__main__":
    main()
