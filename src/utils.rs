use std::path::{Path, PathBuf};
use tokio::fs;
use crate::config::FsRemap;
use anyhow::Result;

pub fn remap_to_worker(path: &Path, remaps: &Option<Vec<FsRemap>>) -> PathBuf {
    if let Some(vec) = remaps {
        vec.iter().fold(path.to_path_buf(), |acc, r| r.map_to_worker(&acc))
    } else {
        path.to_path_buf()
    }
}

pub fn remap_to_master(path: &Path, remaps: &Option<Vec<FsRemap>>) -> PathBuf {
    if let Some(vec) = remaps {
        vec.iter().fold(path.to_path_buf(), |acc, r| r.map_to_master(&acc))
    } else {
        path.to_path_buf()
    }
}

pub async fn copy_preserve_structure(
    original_file: &Path,
    src_file: &Path,
    library_root: &Path,
    dst_dir: &Path,
) -> Result<PathBuf> {
    // Compute the relative path from the library root
    let relative_path = original_file
        .strip_prefix(library_root)
        .map_err(|_| anyhow::anyhow!(
            "File {} is not under library root {}",
            original_file.display(),
            library_root.display()
        ))?;

    // Build the full destination path
    //let dst_path = dst_dir.join(relative_path);
    let dst_path = dst_dir.join(
        relative_path.parent()
            .unwrap_or_else(|| Path::new(""))
            .join(src_file.file_name().unwrap())
    );
    
    // Ensure all parent directories exist
    if let Some(parent) = dst_path.parent() {
        fs::create_dir_all(parent).await?;
    }

    // Copy the file
    fs::copy(src_file, &dst_path).await?;

    Ok(dst_path)
}
