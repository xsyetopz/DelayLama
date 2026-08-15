from __future__ import annotations

import argparse
import json
import shutil
import subprocess
from pathlib import Path
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    # Import siblings directly because this file runs as both script and module.
    from editor_surface import editor_header
    from editor_template import editor_source
    from processor import processor_header
    from processor_runtime import processor_source
else:
    try:
        from .editor_surface import editor_header
        from .editor_template import editor_source
        from .processor import processor_header
        from .processor_runtime import processor_source
    except ImportError:
        from editor_surface import editor_header
        from editor_template import editor_source
        from processor import processor_header
        from processor_runtime import processor_source

MODEL_FILES = (
    Path("src/host/processor.hpp"),
    Path("src/host/processor.cpp"),
    Path("src/editor/interaction.hpp"),
    Path("src/editor/interaction.cpp"),
)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output-dir", type=Path, required=True)
    parser.add_argument("--source-root", type=Path, required=True)
    parser.add_argument("--identity", type=Path, required=True)
    return parser.parse_args()


def load_identity(path: Path) -> dict[str, object]:
    with path.open("r", encoding="utf-8") as stream:
        raw_identity: object = json.load(stream)
    if not isinstance(raw_identity, dict):
        raise TypeError("identity manifest must contain a JSON object")
    identity: dict[str, object] = {}
    for key, value in raw_identity.items():
        if not isinstance(key, str):
            raise TypeError("identity manifest keys must be strings")
        identity[key] = value
    required = (
        "productName",
        "version",
        "bundleIdentifier",
        "publisher",
        "audioUnit",
        "vst3",
    )
    missing = [key for key in required if key not in identity]
    if missing:
        raise ValueError(f"identity manifest missing: {', '.join(missing)}")
    return identity


def validate_model_sources(source_root: Path) -> None:
    missing = [path for path in MODEL_FILES if not (source_root / path).is_file()]
    if missing:
        names = ", ".join(str(path) for path in missing)
        raise FileNotFoundError(f"JUCE model sources are missing: {names}")


def write_if_changed(path: Path, content: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    if path.is_file() and path.read_text(encoding="utf-8") == content:
        return
    path.write_text(content, encoding="utf-8")


def format_source(name: str, content: str, source_root: Path) -> str:
    formatter = shutil.which("clang-format")
    if formatter is None:
        raise RuntimeError(
            "clang-format is required to generate the JUCE adapter sources"
        )
    style_file = source_root / ".clang-format"
    if not style_file.is_file():
        raise FileNotFoundError(
            f"repository clang-format rules are missing: {style_file}"
        )

    owner = "processor" if name.startswith("processor") else "editor"
    result = subprocess.run(
        [
            formatter,
            f"--style=file:{style_file}",
            f"--assume-filename={source_root / 'src' / 'juce' / owner / name}",
        ],
        input=content,
        text=True,
        capture_output=True,
        check=True,
    )
    return result.stdout


def generate(output_dir: Path, source_root: Path, identity_path: Path) -> None:
    identity = load_identity(identity_path)
    validate_model_sources(source_root)
    product_name = identity["productName"]
    if not isinstance(product_name, str) or not product_name.strip():
        raise ValueError("the generated adapter requires a non-empty product identity")

    files = {
        "processor.hpp": processor_header(product_name),
        "processor.cpp": processor_source(),
        "editor.hpp": editor_header(),
        "editor.cpp": editor_source(product_name),
    }
    for name, content in files.items():
        write_if_changed(
            output_dir / name,
            format_source(name, content, source_root),
        )


def main() -> None:
    args = parse_args()
    generate(args.output_dir, args.source_root, args.identity)


if __name__ == "__main__":
    main()
