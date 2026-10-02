use clap::Parser;
use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
    process::Command,
};
#[derive(Parser)]
struct Args {
    #[arg(long)]
    dev: bool,
}
fn collect(root: &Path) -> io::Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        if entry.file_type()?.is_symlink() {
            return Err(io::Error::other("Symlink in package inputs"));
        }
        if entry.file_type()?.is_dir() {
            out.extend(collect(&entry.path())?);
        } else {
            out.push(entry.path());
        }
    }
    Ok(out)
}
fn main() -> io::Result<()> {
    let dev = Args::parse().dev;
    let mut build = Command::new("cargo");
    build.args(["build", "--locked", "--bins"]);
    if !dev {
        build.arg("--release");
    }
    if !build.status()?.success() {
        return Err(io::Error::other("Build failed"));
    }
    if !Command::new(if cfg!(windows) { "npm.cmd" } else { "npm" })
        .args(["--prefix", "frontend", "run", "build"])
        .status()?
        .success()
    {
        return Err(io::Error::other("Frontend build failed"));
    }
    let profile = if dev { "debug" } else { "release" };
    let platform = if cfg!(windows) { "windows" } else { "linux" };
    let arch = std::env::consts::ARCH;
    let name = format!(
        "WebDeck-{}-{platform}-{arch}{}-portable.zip",
        env!("CARGO_PKG_VERSION"),
        if dev { "-dev" } else { "" }
    );
    fs::create_dir_all("dist")?;
    let mut zip = zip::ZipWriter::new(fs::File::create(Path::new("dist").join(&name))?);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    for (bin, label) in [
        ("webdeck", "WebDeck"),
        ("update", "update"),
        ("webdeck-qr", "webdeck-qr"),
        ("console", "console"),
    ] {
        let ext = if cfg!(windows) { ".exe" } else { "" };
        let src = PathBuf::from(format!("target/{profile}/{bin}{ext}"));
        let bytes = if cfg!(unix) && dev {
            let temp =
                std::env::temp_dir().join(format!("webdeck-strip-{}", webdeck::domain::id()));
            fs::copy(&src, &temp)?;
            if !Command::new("strip")
                .args(["--strip-debug"])
                .arg(&temp)
                .status()?
                .success()
            {
                return Err(io::Error::other("Strip failed"));
            }
            let b = fs::read(&temp)?;
            fs::remove_file(temp)?;
            b
        } else {
            fs::read(src)?
        };
        zip.start_file(
            format!("WebDeck/{label}{ext}"),
            options.unix_permissions(0o755),
        )?;
        zip.write_all(&bytes)?;
    }
    for root in ["webdeck", "static", "frontend/dist"] {
        for file in collect(Path::new(root))? {
            zip.start_file(
                format!("WebDeck/{}", file.to_string_lossy().replace('\\', "/")),
                options.unix_permissions(0o644),
            )?;
            zip.write_all(&fs::read(file)?)?;
        }
    }
    zip.finish()?.sync_all()?;
    let path = Path::new("dist").join(&name);
    let digest = webdeck::update::digest(&fs::read(&path)?);
    fs::write(
        format!("{}.sha256", path.display()),
        format!("{digest}  {name}\n"),
    )?;
    println!("{}", path.display());
    Ok(())
}
