use anyhow::{Context, Result};
use clap::Parser;
use percent_encoding::percent_decode_str;
use regex::Regex;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Parser, Debug)]
#[command(name = "md-prune-image")]
#[command(about = "Remove orphaned image files from markdown directories", long_about = None)]
#[command(styles = clap_cargo::style::CLAP_STYLING)]
struct Cli {
    /// Target directory to scan
    #[arg(value_name = "DIRECTORY")]
    directory: PathBuf,

    /// Move orphaned images to system recycle bin (default)
    #[arg(long, group = "action")]
    recycle: bool,

    /// Permanently delete orphaned images
    #[arg(long, group = "action")]
    delete: bool,

    /// Move orphaned images to specified directory
    #[arg(long, group = "action", value_name = "DIR")]
    r#move: Option<PathBuf>,

    /// Image file extensions to consider (comma-separated)
    #[arg(long, default_value = "jpg,jpeg,png,gif,bmp,svg,webp")]
    extensions: String,
}

#[derive(Debug)]
enum Action {
    Delete,
    Recycle,
    Move(PathBuf),
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Validate target directory
    if !cli.directory.exists() {
        anyhow::bail!("Directory does not exist: {}", cli.directory.display());
    }
    if !cli.directory.is_dir() {
        anyhow::bail!("Path is not a directory: {}", cli.directory.display());
    }

    // Determine action (default to recycle if no flag specified)
    let action = if cli.delete {
        Action::Delete
    } else if let Some(ref dir) = cli.r#move {
        Action::Move(dir.clone())
    } else {
        Action::Recycle
    };

    // Canonicalize base directory for consistent path operations
    let base_dir = cli.directory.canonicalize().with_context(|| {
        format!(
            "Failed to canonicalize directory: {}",
            cli.directory.display()
        )
    })?;

    // Scan for orphaned images
    let orphaned_images = scan_for_orphans(&cli)?;

    if orphaned_images.is_empty() {
        return Ok(());
    }

    // Print orphaned images list
    let dir_name = cli
        .directory
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(".");
    for img in &orphaned_images {
        println!("{}/{}", dir_name, display_relative_path(img, &base_dir));
    }

    // Execute removal action
    execute_action(&action, &orphaned_images, &base_dir)?;

    Ok(())
}

fn scan_for_orphans(cli: &Cli) -> Result<Vec<PathBuf>> {
    // Step 1: Find all image files
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

    // Step 3: Find orphaned images (images not referenced in any markdown)
    let orphaned: Vec<PathBuf> = all_images.difference(&referenced_images).cloned().collect();

    Ok(orphaned)
}

fn walk_files(directory: &PathBuf) -> impl Iterator<Item = walkdir::DirEntry> {
    WalkDir::new(directory)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|entry| entry.file_type().is_file())
}

fn extract_image_references(markdown_path: &Path, base_dir: &Path) -> Result<HashSet<PathBuf>> {
    let content = fs::read_to_string(markdown_path)
        .with_context(|| format!("Failed to read markdown file: {}", markdown_path.display()))?;

    let mut references = HashSet::new();
    let markdown_dir = markdown_path.parent().unwrap_or(base_dir);

    // Regex patterns for markdown image syntax
    // ![alt](path) and ![alt](path "title")
    let img_pattern = Regex::new(r#"!\[.*?]\(([^)]+?)(?:\s+["'].*?["'])?\)"#)?;

    // HTML img tags: <img src="path">
    let html_pattern = Regex::new(r#"<img[^>]+src=["']([^"']+)["']"#)?;

    // Extract from markdown syntax
    for cap in img_pattern.captures_iter(&content) {
        if let Some(path_match) = cap.get(1) {
            let img_path = path_match.as_str().trim();

            // Skip URLs (http://, https://, //, data:, etc.)
            if img_path.starts_with("http://")
                || img_path.starts_with("https://")
                || img_path.starts_with("//")
                || img_path.starts_with("data:")
            {
                continue;
            }

            if let Some(resolved) = resolve_image_path(img_path, markdown_dir, base_dir) {
                references.insert(resolved);
            }
        }
    }

    // Extract from HTML syntax
    for cap in html_pattern.captures_iter(&content) {
        if let Some(path_match) = cap.get(1) {
            let img_path = path_match.as_str().trim();

            // Skip URLs
            if img_path.starts_with("http://")
                || img_path.starts_with("https://")
                || img_path.starts_with("//")
                || img_path.starts_with("data:")
            {
                continue;
            }

            if let Some(resolved) = resolve_image_path(img_path, markdown_dir, base_dir) {
                references.insert(resolved);
            }
        }
    }

    Ok(references)
}

fn resolve_image_path(img_path: &str, markdown_dir: &Path, base_dir: &Path) -> Option<PathBuf> {
    // 1. Try URL decoding (fallback to original on failure)
    let decoded_path = percent_decode_str(img_path)
        .decode_utf8()
        .map(|s| s.into_owned())
        .unwrap_or_else(|_| img_path.to_string());

    // 2. Remove fragment (#) and query string (?)
    let clean_path = decoded_path
        .split('#')
        .next()
        .and_then(|s| s.split('?').next())
        .unwrap_or(&decoded_path);

    // 3. Try decoded path first
    try_resolve_path(clean_path, markdown_dir, base_dir).or_else(|| {
        // 4. Fallback: try original path (filename may already be decoded)
        if clean_path != img_path {
            let clean_original = img_path
                .split('#')
                .next()
                .and_then(|s| s.split('?').next())
                .unwrap_or(img_path);
            try_resolve_path(clean_original, markdown_dir, base_dir)
        } else {
            None
        }
    })
}

fn try_resolve_path(img_path: &str, markdown_dir: &Path, base_dir: &Path) -> Option<PathBuf> {
    // Try to resolve the path relative to the markdown file's directory
    let relative_to_md = markdown_dir.join(img_path);
    if let Ok(canonical) = relative_to_md.canonicalize() {
        // Check if it's within the base directory
        if canonical.starts_with(base_dir.canonicalize().ok()?) {
            return Some(canonical);
        }
    }

    // Try to resolve relative to base directory
    let relative_to_base = base_dir.join(img_path);
    if let Ok(canonical) = relative_to_base.canonicalize()
        && canonical.starts_with(base_dir.canonicalize().ok()?)
    {
        return Some(canonical);
    }

    // Try as absolute path
    let abs_path = PathBuf::from(img_path);
    if abs_path.is_absolute()
        && let Ok(canonical) = abs_path.canonicalize()
        && canonical.starts_with(base_dir.canonicalize().ok()?)
    {
        return Some(canonical);
    }

    None
}

fn execute_action(action: &Action, orphaned_images: &[PathBuf], _base_dir: &Path) -> Result<()> {
    match action {
        Action::Delete => {
            for img in orphaned_images {
                fs::remove_file(img)
                    .with_context(|| format!("Failed to delete: {}", img.display()))?;
            }
            println!("Deleted: {} image(s)", orphaned_images.len());
        }
        Action::Recycle => {
            for img in orphaned_images {
                trash::delete(img)
                    .with_context(|| format!("Failed to move to recycle bin: {}", img.display()))?;
            }
            println!("Recycled: {} image(s)", orphaned_images.len());
        }
        Action::Move(move_to_dir) => {
            // Create target directory if it doesn't exist
            if !move_to_dir.exists() {
                fs::create_dir_all(move_to_dir).with_context(|| {
                    format!("Failed to create directory: {}", move_to_dir.display())
                })?;
            }

            for img in orphaned_images {
                if let Some(filename) = img.file_name() {
                    let target = move_to_dir.join(filename);

                    // Handle filename conflicts
                    let final_target = if target.exists() {
                        generate_unique_filename(&target)
                    } else {
                        target
                    };

                    fs::rename(img, &final_target).with_context(|| {
                        format!(
                            "Failed to move {} to {}",
                            img.display(),
                            final_target.display()
                        )
                    })?;
                }
            }
            println!("Moved: {} image(s)", orphaned_images.len());
        }
    }

    Ok(())
}

fn generate_unique_filename(path: &Path) -> PathBuf {
    let parent = path.parent().unwrap();
    let stem = path.file_stem().unwrap().to_string_lossy();
    let ext = path
        .extension()
        .map(|e| e.to_string_lossy())
        .unwrap_or_default();

    let mut counter = 1;
    loop {
        let new_name = if ext.is_empty() {
            format!("{}_{}", stem, counter)
        } else {
            format!("{}_{}.{}", stem, counter, ext)
        };

        let new_path = parent.join(new_name);
        if !new_path.exists() {
            return new_path;
        }
        counter += 1;
    }
}

fn display_relative_path(path: &Path, base_dir: &Path) -> String {
    path.strip_prefix(base_dir)
        .map(|p| p.display().to_string().replace('\\', "/"))
        .unwrap_or_else(|_| path.display().to_string().replace('\\', "/"))
}
