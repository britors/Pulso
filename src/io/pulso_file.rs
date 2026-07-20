use crate::model::Document;
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs::File,
    io::{Read, Write},
    path::Path,
};
use zip::{write::SimpleFileOptions, ZipArchive, ZipWriter};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Meta {
    app: String,
    format_version: u32,
}

pub fn save(path: &Path, document: &Document, media: &HashMap<String, Vec<u8>>) -> Result<()> {
    let file = File::create(path).with_context(|| format!("cannot create {}", path.display()))?;
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    zip.start_file("document.json", options)?;
    zip.write_all(&serde_json::to_vec_pretty(document)?)?;
    zip.start_file("meta.json", options)?;
    zip.write_all(&serde_json::to_vec_pretty(&Meta {
        app: "Pulso".into(),
        format_version: 1,
    })?)?;
    for (name, bytes) in media {
        zip.start_file(format!("media/{name}"), options)?;
        zip.write_all(bytes)?;
    }
    zip.finish()?;
    Ok(())
}

pub fn load(path: &Path) -> Result<(Document, HashMap<String, Vec<u8>>)> {
    let mut zip = ZipArchive::new(File::open(path)?)?;
    let document: Document = {
        let mut entry = zip
            .by_name("document.json")
            .context("document.json missing")?;
        let mut data = Vec::new();
        entry.read_to_end(&mut data)?;
        serde_json::from_slice(&data)?
    };
    if document.schema_version != 1 {
        bail!("unsupported schema version {}", document.schema_version);
    }
    let mut media = HashMap::new();
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i)?;
        let name = entry.name().to_owned();
        if let Some(short) = name.strip_prefix("media/").filter(|s| !s.is_empty()) {
            let mut data = Vec::new();
            entry.read_to_end(&mut data)?;
            media.insert(short.to_owned(), data);
        }
    }
    Ok((document, media))
}
