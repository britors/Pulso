#[path = "../src/model/mod.rs"]
mod model;
#[path = "../src/io/pulso_file.rs"]
mod pulso_file;

use model::{Document, Element, ShapeType, TextAttrs, TextRun};
use std::collections::HashMap;

#[test]
fn model_json_roundtrip_is_lossless_and_english() {
    let mut document = Document::default();
    let mut text = Element::text(80.0, 60.0);
    if let model::ElementKind::Text { props } = &mut text.kind {
        props.blocks[0].runs = vec![TextRun {
            text: "Apresentação sem HTML".into(),
            attrs: TextAttrs {
                bold: true,
                italic: true,
                underline: false,
                strikethrough: false,
                color: Some("#FF6F61".into()),
            },
        }];
    }
    text.frame.rotation = 12.5;
    document.slides[0].elements.push(text);
    document.slides[0]
        .elements
        .push(Element::shape(ShapeType::Ellipse, 400.0, 240.0));
    let json = serde_json::to_string_pretty(&document).unwrap();
    assert!(json.contains("\"schemaVersion\": 1"));
    assert!(json.contains("\"type\": \"text\""));
    assert!(!json.to_lowercase().contains("<html"));
    let decoded: Document = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded, document);
}

#[test]
fn pulso_zip_roundtrip_keeps_media() {
    let document = Document::default();
    let media = HashMap::from([("abc123.png".into(), vec![1, 2, 3, 4])]);
    let temporary = tempfile::NamedTempFile::new().unwrap();
    pulso_file::save(temporary.path(), &document, &media).unwrap();
    let (loaded, loaded_media) = pulso_file::load(temporary.path()).unwrap();
    assert_eq!(loaded, document);
    assert_eq!(loaded_media, media);
}

#[test]
fn commands_undo_and_redo_mutations() {
    use model::commands::{AddElement, ChangeFrames};
    use model::undo::UndoStack;
    let mut document = Document::default();
    let element = Element::shape(ShapeType::Rect, 10.0, 20.0);
    let id = element.id.clone();
    let mut history = UndoStack::default();
    history.execute(Box::new(AddElement { slide: 0, element }), &mut document);
    assert_eq!(document.slides[0].elements.len(), 1);
    let before = document.slides[0].elements[0].frame;
    let mut after = before;
    after.x = 99.0;
    history.execute(
        Box::new(ChangeFrames {
            slide: 0,
            changes: vec![(id, before, after)],
        }),
        &mut document,
    );
    assert_eq!(document.slides[0].elements[0].frame.x, 99.0);
    assert!(history.undo(&mut document));
    assert_eq!(document.slides[0].elements[0].frame.x, 10.0);
    assert!(history.redo(&mut document));
    assert_eq!(document.slides[0].elements[0].frame.x, 99.0);
}

#[test]
fn slide_reordering_preserves_identity_and_is_undoable() {
    use model::commands::ReorderSlide;
    use model::undo::UndoStack;
    let mut document = Document::default();
    document
        .slides
        .push(model::Slide::new(model::Background::default()));
    document
        .slides
        .push(model::Slide::new(model::Background::default()));
    let ids = document
        .slides
        .iter()
        .map(|slide| slide.id.clone())
        .collect::<Vec<_>>();
    let mut history = UndoStack::default();
    history.execute(Box::new(ReorderSlide { from: 0, to: 2 }), &mut document);
    assert_eq!(document.slides[2].id, ids[0]);
    assert!(history.undo(&mut document));
    assert_eq!(
        document
            .slides
            .iter()
            .map(|slide| &slide.id)
            .collect::<Vec<_>>(),
        ids.iter().collect::<Vec<_>>()
    );
}
