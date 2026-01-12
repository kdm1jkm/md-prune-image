use std::path::Path;

pub fn display_relative_path(path: &Path, base_dir: &Path) -> String {
    path.strip_prefix(base_dir)
        .map(|p| p.display().to_string().replace('\\', "/"))
        .unwrap_or_else(|_| path.display().to_string().replace('\\', "/"))
}
