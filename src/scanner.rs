use crate::cli::Cli;
use crate::error::Result;
use crate::parser::extract_image_references;
use std::collections::HashSet;
use std::path::PathBuf;
use walkdir::WalkDir;

pub fn scan_for_orphans(cli: &Cli) -> Result<Vec<PathBuf>> {
    let image_extensions: HashSet<String> = cli
        .extensions
        .split(',')
        .map(|s| s.trim().to_lowercase())
        .collect();

    let all_images = walk_files(&cli.directory)
        .filter(|entry| {
            entry
                .path()
                .extension()
                .map(|ext| image_extensions.contains(&ext.to_string_lossy().to_lowercase()))
                .unwrap_or(false)
        })
        .filter_map(|entry| entry.path().canonicalize().ok())
        .collect::<HashSet<PathBuf>>();

    let referenced_images: HashSet<PathBuf> = WalkDir::new(&cli.directory)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|entry| entry.file_type().is_file())
        .filter(|entry| {
            entry
                .path()
                .extension()
                .map(|ext| ext.eq_ignore_ascii_case("md") || ext.eq_ignore_ascii_case("markdown"))
                .unwrap_or(false)
        })
        .filter_map(|entry| extract_image_references(entry.path(), &cli.directory).ok())
        .flatten()
        .collect();

    let orphaned: Vec<PathBuf> = all_images.difference(&referenced_images).cloned().collect();

    Ok(orphaned)
}

pub(crate) fn walk_files(directory: &PathBuf) -> impl Iterator<Item = walkdir::DirEntry> {
    WalkDir::new(directory)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|entry| entry.file_type().is_file())
}
