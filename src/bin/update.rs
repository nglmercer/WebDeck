use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};
#[derive(Parser)]
#[command(version)]
struct Args {
    #[command(subcommand)]
    action: Action,
}
#[derive(Subcommand)]
enum Action {
    Check,
    Fetch {
        #[arg(long)]
        destination: PathBuf,
        #[arg(long)]
        backup: PathBuf,
    },
    Install {
        #[arg(long)]
        archive: PathBuf,
        #[arg(long)]
        sha256: String,
        #[arg(long)]
        destination: PathBuf,
        #[arg(long)]
        backup: PathBuf,
    },
    Rollback {
        #[arg(long)]
        destination: PathBuf,
        #[arg(long)]
        backup: PathBuf,
    },
}
fn install(bytes: &[u8], digest: &str, destination: &Path, backup: &Path) -> std::io::Result<()> {
    let stage = std::env::temp_dir().join(format!("webdeck-stage-{}", webdeck::domain::id()));
    webdeck::update::stage(bytes, digest, &stage)?;
    let result = webdeck::update::install(&stage, destination, backup);
    let _ = std::fs::remove_dir_all(stage);
    result
}
#[tokio::main]
async fn main() {
    let result = match Args::parse().action {
        Action::Check => webdeck::update::latest().await.map(|release| {
            if let Some(r) = release {
                println!("{}\n{}\n{}", r.version, r.url, r.digest);
            } else {
                println!("No verified newer v2 prerelease is available.");
            }
        }),
        Action::Fetch {
            destination,
            backup,
        } => {
            async {
                let r = webdeck::update::latest()
                    .await?
                    .ok_or_else(|| std::io::Error::other("No verified release"))?;
                let b = webdeck::update::download(&r.url).await?;
                install(&b, &r.digest, &destination, &backup)
            }
            .await
        }
        Action::Install {
            archive,
            sha256,
            destination,
            backup,
        } => std::fs::read(archive)
            .and_then(|b| install(&b, &format!("sha256:{sha256}"), &destination, &backup)),
        Action::Rollback {
            destination,
            backup,
        } => webdeck::update::rollback(&destination, &backup),
    };
    if result.is_err() {
        eprintln!("Update failed. Preserve the backup for recovery.");
        std::process::exit(1);
    }
}
