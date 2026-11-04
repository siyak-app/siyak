//! Build script: generates the Tauri context (config, capabilities, icons) and
//! the `allow-*` permissions for the app's own commands.

include!("src/commands/names.rs");

/// Lists the citation style templates (`src/export/citation/*.tmpl`) in a file
/// that `export/citation.rs` includes, so adding a style needs no code change.
fn write_citation_styles() {
    use std::fmt::Write as _;

    const DIR: &str = "src/export/citation";
    println!("cargo:rerun-if-changed={DIR}");
    let root = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
    let mut styles: Vec<String> = std::fs::read_dir(DIR)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter_map(|name| name.strip_suffix(".tmpl").map(str::to_owned))
        .collect();
    styles.sort();
    let mut out = String::from(
        "/// The style templates: `(id, file contents)`.\nconst STYLE_FILES: &[(&str, &str)] = &[\n",
    );
    for id in styles {
        let path = format!("{root}/{DIR}/{id}.tmpl");
        let _ = writeln!(out, "    ({id:?}, include_str!({path:?})),");
    }
    out.push_str("];\n");
    let target = std::path::Path::new(&std::env::var("OUT_DIR").unwrap_or_default())
        .join("citation_styles.rs");
    if let Err(err) = std::fs::write(&target, out) {
        panic!("writing {}: {err}", target.display());
    }
}

fn main() {
    write_citation_styles();
    // Listing the commands makes Tauri deny any command not granted by a
    // capability, instead of allowing every registered command by default.
    let manifest = tauri_build::AppManifest::new().commands(COMMAND_NAMES);
    if let Err(err) = tauri_build::try_build(tauri_build::Attributes::new().app_manifest(manifest))
    {
        panic!("tauri-build failed: {err:#}");
    }
}
