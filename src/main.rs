use clap::{Parser, Subcommand};
use anyhow::Result;

mod ops;
mod fsx;

#[derive(Parser, Debug)]
#[command(name = "safe_backup_rust", version, about = "Pure Rust backup/restore/delete tool")]
struct Cli {
    /// Custom backup directory (default: ./backups)
    #[arg(global = true, long, value_name = "DIR")]
    backup_dir: Option<std::path::PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Backup a file into the backup directory
    Backup {
        /// Path to the file to backup
        path: std::path::PathBuf,
    },
    /// Restore the latest (or specific) backup back to original location
    Restore {
        /// Original path of the file
        path: std::path::PathBuf,

        /// Optional backup ID to restore (otherwise latest is used)
        #[arg(long)]
        id: Option<String>,
    },
    /// Delete a specific backup by ID
    Delete {
        /// Backup ID (from `list` output)
        id: String,
    },
    /// List backups for a given original file
    List {
        /// Original path of the file
        path: std::path::PathBuf,
    },
    View {
        /// Path to the original file
        path: std::path::PathBuf,

        /// Optional backup ID (if you want to view a backup instead of the original)
        #[arg(long)]
        id: Option<String>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let backup_dir = cli.backup_dir.unwrap_or_else(|| std::path::PathBuf::from("backups"));
    std::fs::create_dir_all(&backup_dir)?;

    match cli.command {
        Commands::Backup { path } => ops::backup(&path, &backup_dir)?,
        Commands::Restore { path, id } => ops::restore(&path, id.as_deref(), &backup_dir)?,
        Commands::Delete { id } => ops::delete(&id, &backup_dir)?,
        Commands::List { path } => ops::list(&path, &backup_dir)?,
        Commands::View { path, id } => ops::view(&path, id.as_deref(), &backup_dir)?,
    }

    Ok(())
}
