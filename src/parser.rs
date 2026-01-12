use crate::error::{Error, Result};
use percent_encoding::percent_decode_str;
use regex::Regex;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

pub fn extract_image_references(markdown_path: &Path, base_dir: &Path) -> Result<HashSet<PathBuf>> {
    let content = fs::read_to_string(markdown_path).map_err(|source| Error::ReadFile {
        path: markdown_path.to_path_buf(),
        source,
    })?;

    let mut references = HashSet::new();
    let markdown_dir = markdown_path.parent().unwrap_or(base_dir);

    // Regex for markdown image syntax: ![alt](path) and ![alt](path "title")
    let img_pattern = Regex::new(r#"!\[.*?]\(([^)]+?)(?:\s+["'].*?["'])?\)"#)?;

    // Regex for HTML img tags: <img src="path">
    let html_pattern = Regex::new(r#"<img[^>]+src=["']([^"']+)["']"#)?;

    for cap in img_pattern.captures_iter(&content) {
        if let Some(path_match) = cap.get(1) {
            let img_path = path_match.as_str().trim();

            if is_url(img_path) {
                continue;
            }

            if let Some(resolved) = resolve_image_path(img_path, markdown_dir, base_dir) {
                references.insert(resolved);
            }
        }
    }

    for cap in html_pattern.captures_iter(&content) {
        if let Some(path_match) = cap.get(1) {
            let img_path = path_match.as_str().trim();

            if is_url(img_path) {
                continue;
            }

            if let Some(resolved) = resolve_image_path(img_path, markdown_dir, base_dir) {
                references.insert(resolved);
            }
        }
    }

    Ok(references)
}

fn is_url(path: &str) -> bool {
    path.starts_with("http://")
        || path.starts_with("https://")
        || path.starts_with("//")
        || path.starts_with("data:")
}

pub(crate) fn resolve_image_path(
    img_path: &str,
    markdown_dir: &Path,
    base_dir: &Path,
) -> Option<PathBuf> {
    let decoded_path = percent_decode_str(img_path)
        .decode_utf8()
        .map(|s| s.into_owned())
        .unwrap_or_else(|_| img_path.to_string());

    let clean_path = decoded_path
        .split('#')
        .next()
        .and_then(|s| s.split('?').next())
        .unwrap_or(&decoded_path);

    try_resolve_path(clean_path, markdown_dir, base_dir).or_else(|| {
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
    let relative_to_md = markdown_dir.join(img_path);
    if let Ok(canonical) = relative_to_md.canonicalize()
        && canonical.starts_with(base_dir.canonicalize().ok()?)
    {
        return Some(canonical);
    }

    let relative_to_base = base_dir.join(img_path);
    if let Ok(canonical) = relative_to_base.canonicalize()
        && canonical.starts_with(base_dir.canonicalize().ok()?)
    {
        return Some(canonical);
    }

    let abs_path = PathBuf::from(img_path);
    if abs_path.is_absolute()
        && let Ok(canonical) = abs_path.canonicalize()
        && canonical.starts_with(base_dir.canonicalize().ok()?)
    {
        return Some(canonical);
    }

    None
}
