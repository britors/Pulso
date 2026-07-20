use crate::model::{ElementKind, TextBlock, TextProps, TextRun};
use gtk::prelude::*;

pub fn plain_text(props: &TextProps) -> String {
    props
        .blocks
        .iter()
        .map(|b| b.runs.iter().map(|r| r.text.as_str()).collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}
pub fn update_from_plain(kind: &mut ElementKind, text: &str) {
    if let ElementKind::Text { props } = kind {
        let template = props.blocks.first().cloned().unwrap_or_default();
        props.blocks = text
            .split('\n')
            .map(|line| TextBlock {
                runs: vec![TextRun {
                    text: line.into(),
                    attrs: template
                        .runs
                        .first()
                        .map(|r| r.attrs.clone())
                        .unwrap_or_default(),
                }],
                ..template.clone()
            })
            .collect();
    }
}
pub fn create(text: &str) -> gtk::TextView {
    let view = gtk::TextView::builder()
        .wrap_mode(gtk::WrapMode::WordChar)
        .css_classes(["card"])
        .build();
    view.buffer().set_text(text);
    view
}
