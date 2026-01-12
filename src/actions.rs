use crate::cli::Action;
use crate::error::{Error, Result};
use std::fs;
use std::path::{Path, PathBuf};

pub fn execute_action(
    action: &Action,
    orphaned_images: &[PathBuf],
    _base_dir: &Path,
) -> Result<()> {
    match action {
        Action::Delete => {
            for img in orphaned_images {
                fs::remove_file(img).map_err(|source| Error::DeleteFile {
                    path: img.clone(),
                    source,
                })?;
            }
            println!("Deleted: {} image(s)", orphaned_images.len());
        }
        Action::Recycle => {
            for img in orphaned_images {
                trash::delete(img).map_err(|source| Error::RecycleFile {
                    path: img.clone(),
                    source,
                })?;
            }
            println!("Recycled: {} image(s)", orphaned_images.len());
        }
        Action::Move(move_to_dir) => {
            if !move_to_dir.exists() {
                fs::create_dir_all(move_to_dir).map_err(|source| Error::CreateDirectory {
                    path: move_to_dir.clone(),
                    source,
                })?;
            }

            for img in orphaned_images {
                if let Some(filename) = img.file_name() {
                    let target = move_to_dir.join(filename);

                    let final_target = if target.exists() {
                        generate_unique_filename(&target)
                    } else {
                        target
                    };

                    fs::rename(img, &final_target).map_err(|source| Error::MoveFile {
                        from: img.clone(),
                        to: final_target.clone(),
                        source,
                    })?;
                }
            }
            println!("Moved: {} image(s)", orphaned_images.len());
        }
    }

    Ok(())
}

pub(crate) fn generate_unique_filename(path: &Path) -> PathBuf {
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
