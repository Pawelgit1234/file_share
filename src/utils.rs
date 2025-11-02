use std::path::PathBuf;

use tokio::fs::File;
use tokio::io::AsyncReadExt;

use crate::config::CHUNK_SIZE;

pub async fn hash_file(path: &PathBuf) -> anyhow::Result<String> {
    let mut hasher = blake3::Hasher::new();
    let mut tmp = File::open(path).await?;
    let mut buf = vec![0u8; CHUNK_SIZE];
    loop {
        let n = tmp.read(&mut buf).await?;
        if n == 0 { break; }
        hasher.update(&buf[..n]);
    }
    let hash = hasher.finalize().to_hex().to_string();
    Ok(hash)
}

pub async fn get_file_length(file: &File) -> anyhow::Result<u64> {
    let meta = file.metadata().await?;
    Ok(meta.len())
}