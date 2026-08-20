//! Writes LV2 parameter data for the hand-written parameter implementation.

use std::{env, fmt::Write as _, fs};

#[path = "src/plugin/parameter.rs"]
pub mod parameter;

use parameter::PluginParameter;

fn parameter_sidecar() -> Result<String, std::fmt::Error> {
    let mut sidecar = String::new();
    writeln!(sidecar, "struct = \"PluginParams\"")?;
    writeln!(sidecar, "scheme = \"ordinal\"")?;
    for (ordinal, parameter) in PluginParameter::ALL.into_iter().enumerate() {
        if parameter.index() != ordinal
            || PluginParameter::from_id(parameter.id()) != Some(parameter)
        {
            return Err(std::fmt::Error);
        }
        writeln!(sidecar)?;
        writeln!(sidecar, "[[param]]")?;
        writeln!(sidecar, "id = {}", parameter.id())?;
        writeln!(sidecar, "field = \"{}\"", parameter.field_name())?;
        writeln!(sidecar, "name = \"{}\"", parameter.display_name())?;
        writeln!(sidecar, "kind = \"Float\"")?;
        writeln!(sidecar, "range = \"linear(0, 1)\"")?;
        writeln!(sidecar, "default = {}", parameter.default())?;
    }
    Ok(sidecar)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/plugin/parameter.rs");

    let package_name = env::var("CARGO_PKG_NAME")?;
    let truce_manifest = truce_build::find_truce_toml()?;
    let package_root = truce_manifest
        .parent()
        .ok_or_else(|| std::io::Error::other("root truce.toml must have a parent directory"))?;
    let sidecar_directory = truce_build::target_dir(package_root)
        .join("lv2-meta")
        .join(package_name);

    fs::create_dir_all(&sidecar_directory)?;
    fs::write(
        sidecar_directory.join("PluginParams.params.toml"),
        parameter_sidecar()?,
    )?;

    Ok(())
}
