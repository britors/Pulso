use crate::{io::pulso_file, model::Document};
use anyhow::Result;
use std::{collections::HashMap, fs, path::PathBuf, time::SystemTime};

pub fn directory() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("pulso/recovery")
}
pub fn snapshot_path() -> PathBuf {
    directory().join("autosave.pulso")
}
pub fn save(document: &Document, media: &HashMap<String, Vec<u8>>) -> Result<()> {
    fs::create_dir_all(directory())?;
    pulso_file::save(&snapshot_path(), document, media)
}
pub fn latest() -> Option<PathBuf> {
    let path = snapshot_path();
    path.metadata()
        .ok()?
        .modified()
        .ok()
        .filter(|t| *t <= SystemTime::now())
        .map(|_| path)
}
pub fn clear() {
    let _ = fs::remove_file(snapshot_path());
}
