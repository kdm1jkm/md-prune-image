use clap::Parser;
use std::process;

use md_prune_image::{Cli, Error, Result, display_relative_path, execute_action, scan_for_orphans};

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {e}");
        process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();

    if !cli.directory.exists() {
        return Err(Error::DirectoryNotFound(cli.directory.clone()));
    }
    if !cli.directory.is_dir() {
        return Err(Error::NotADirectory(cli.directory.clone()));
    }

    let action = cli.action();

    let base_dir = cli
        .directory
        .canonicalize()
        .map_err(|source| Error::CanonicalizePath {
            path: cli.directory.clone(),
            source,
        })?;

    let orphaned_images = scan_for_orphans(&cli)?;

    if orphaned_images.is_empty() {
        return Ok(());
    }

    let dir_name = cli
        .directory
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(".");

    for img in &orphaned_images {
        println!("{}/{}", dir_name, display_relative_path(img, &base_dir));
    }

    execute_action(&action, &orphaned_images, &base_dir)?;

    Ok(())
}
