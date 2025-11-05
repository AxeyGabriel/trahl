use std::path::{Path, PathBuf};
use tokio::fs;
use uuid::Uuid;
use crate::config::FsRemap;
use anyhow::Result;
use std::fs::File;
use std::io::{BufReader, Read};
use xxhash_rust::xxh3::Xxh3;

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

pub fn uuid_to_u128(value: Uuid) -> u128 {
    u128::from_be_bytes(*value.as_bytes())
}

pub fn u128_to_uuid(value: u128) -> Uuid {
    Uuid::from_bytes(value.to_be_bytes())
}

pub fn chunked_hash(path: impl AsRef<Path>) -> Result<String> {
    const CHUNK_SIZE: usize = 32 * 1024 * 1024; // 32 MB buffer

    let path = path.as_ref();
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let mut buffer = vec![0u8; CHUNK_SIZE];
    let mut hasher = Xxh3::new();

    loop {
        let n = reader.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }

    Ok(format!("{:032x}", hasher.digest128()))
}
