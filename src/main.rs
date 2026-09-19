use std::fs;
use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};
use dockergen::{detect, generate, walk};

#[derive(Parser)]
#[command(
    name = "dockergen",
    about = "Generates a working multi-stage Dockerfile from your actual project"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Print the detected stack without generating anything.
    Detect { path: PathBuf },
    /// Generate a Dockerfile for the project at `path`.
    Generate {
        path: PathBuf,
        #[arg(long, default_value = "Dockerfile")]
        out: PathBuf,
    },
}

fn detect_at(path: &Path) -> anyhow::Result<detect::Stack> {
    let names = walk::list_top_level_names(path)?;
    detect::detect_stack(&names).ok_or_else(|| anyhow::anyhow!("couldn't detect a stack at {} (looked for Cargo.toml, go.mod, package.json, pyproject.toml, requirements.txt)", path.display()))
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Detect { path } => {
            let stack = detect_at(&path)?;
            println!("{}", stack.name());
        }
        Command::Generate { path, out } => {
            let stack = detect_at(&path)?;
            let package_name = if stack == detect::Stack::Rust {
                walk::read_cargo_toml(&path).and_then(|s| detect::parse_cargo_package_name(&s))
            } else {
                None
            };
            let dockerfile = generate::generate_dockerfile(stack, package_name.as_deref());
            fs::write(&out, &dockerfile)
                .map_err(|e| anyhow::anyhow!("writing {}: {e}", out.display()))?;
            println!("detected {} -> wrote {}", stack.name(), out.display());
        }
    }
    Ok(())
}
