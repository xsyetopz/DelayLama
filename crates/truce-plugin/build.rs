//! Generates Truce LV2 parameter metadata for the manual parameter implementation.

use std::{env, fs};

const PARAMETER_SIDECAR: &str = r#"struct = "PluginParams"
scheme = "ordinal"

[[param]]
id = 0
field = "vowel"
name = "Vowel"
kind = "Float"
range = "linear(0, 1)"
default = 0.5

[[param]]
id = 1
field = "port_time"
name = "Portamento"
kind = "Float"
range = "linear(0, 1)"
default = 0.5

[[param]]
id = 2
field = "delay_mix"
name = "Delay"
kind = "Float"
range = "linear(0, 1)"
default = 0.8

[[param]]
id = 3
field = "voice"
name = "Voice"
kind = "Float"
range = "linear(0, 1)"
default = 0.5
"#;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/runtime.rs");

    let package_name = env::var("CARGO_PKG_NAME")?;
    let truce_manifest = truce_build::find_truce_toml()?;
    let workspace_root = truce_manifest.parent().ok_or_else(|| {
        std::io::Error::other("workspace truce.toml must have a parent directory")
    })?;
    let sidecar_directory = truce_build::target_dir(workspace_root)
        .join("lv2-meta")
        .join(package_name);

    fs::create_dir_all(&sidecar_directory)?;
    fs::write(
        sidecar_directory.join("PluginParams.params.toml"),
        PARAMETER_SIDECAR,
    )?;

    Ok(())
}
