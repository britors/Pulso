use crate::{canvas::render::render_slide, model::Document};
use anyhow::Result;
use cairo::{Context, PdfSurface};
use std::{collections::HashMap, path::Path};

pub fn export(document: &Document, media: &HashMap<String, Vec<u8>>, path: &Path) -> Result<()> {
    let surface = PdfSurface::new(
        document.slide_size.width as f64,
        document.slide_size.height as f64,
        path,
    )?;
    let context = Context::new(&surface)?;
    for (index, slide) in document.slides.iter().enumerate() {
        render_slide(&context, slide, Some(media), None)?;
        if index + 1 < document.slides.len() {
            context.show_page()?;
        }
    }
    surface.finish();
    Ok(())
}
