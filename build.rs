use std::{env, fs, path::Path, process::Command};

fn files(root: &Path) -> Vec<std::path::PathBuf> {
    let mut result = Vec::new();
    for entry in fs::read_dir(root).expect("Read frontend assets") {
        let entry = entry.expect("Read asset entry");
        let kind = entry.file_type().expect("Read asset type");
        assert!(
            !kind.is_symlink(),
            "Symlinks are not allowed in embedded assets"
        );
        if kind.is_dir() {
            result.extend(files(&entry.path()));
        } else if kind.is_file() {
            result.push(entry.path());
        }
    }
    result.sort();
    result
}

fn main() {
    println!("cargo:rustc-check-cfg=cfg(webdeck_embedded_frontend)");
    println!("cargo:rerun-if-changed=build.rs");
    // Cargo exposes the target profile's assertions, independently of debug symbols.
    // Development builds neither run npm nor compile frontend assets into Rust.
    if env::var_os("CARGO_CFG_DEBUG_ASSERTIONS").is_some() {
        return;
    }
    println!("cargo:rustc-cfg=webdeck_embedded_frontend");
    let root = std::path::PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    for input in [
        "frontend/src",
        "frontend/public",
        "frontend/index.html",
        "frontend/package.json",
        "frontend/package-lock.json",
        "frontend/vite.config.ts",
        "frontend/tsconfig.json",
        "contracts/v2.schema.json",
        "static",
        "webdeck/translations",
    ] {
        if root.join(input).exists() {
            println!("cargo:rerun-if-changed={input}");
        }
    }
    let output = Command::new(if cfg!(windows) { "npm.cmd" } else { "npm" })
        .args(["run", "build", "--prefix", "frontend"])
        .current_dir(&root)
        .output()
        .expect("Release builds require npm. Install frontend dependencies with npm ci --prefix frontend.");
    assert!(
        output.status.success(),
        "Release frontend build failed. Run npm ci --prefix frontend.\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let mut source =
        String::from("fn embedded_file(path: &str) -> Option<&'static [u8]> {\n match path {\n");
    for (directory, prefix) in [
        ("frontend/dist", ""),
        ("static", "/static"),
        ("webdeck/translations", "/translations"),
    ] {
        let directory = root.join(directory);
        for file in files(&directory) {
            let relative = file
                .strip_prefix(&directory)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            let url = if relative == "index.html" && prefix.is_empty() {
                "/".to_owned()
            } else {
                format!("{prefix}/{relative}")
            };
            source.push_str(&format!(
                "{url:?} => Some(include_bytes!({:?})),\n",
                file.to_str().expect("UTF-8 asset path")
            ));
        }
    }
    source.push_str("_ => None,\n }\n}\n");
    let languages: Vec<_> = files(&root.join("webdeck/translations"))
        .into_iter()
        .filter(|path| path.extension().is_some_and(|ext| ext == "lang"))
        .map(|path| path.file_stem().unwrap().to_str().unwrap().to_owned())
        .collect();
    source.push_str(&format!(
        "pub(super) const EMBEDDED_LANGUAGES: &[&str] = &{languages:?};\n"
    ));
    fs::write(
        std::path::PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("frontend.rs"),
        source,
    )
    .unwrap();
}
