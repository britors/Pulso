use nanoid::nanoid;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const STAGE_WIDTH: f64 = 1280.0;
pub const STAGE_HEIGHT: f64 = 720.0;

fn id(prefix: &str) -> String {
    format!("{prefix}_{}", nanoid!(6))
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Document {
    pub schema_version: u32,
    pub theme: Theme,
    pub slide_size: SlideSize,
    pub slides: Vec<Slide>,
}

impl Default for Document {
    fn default() -> Self {
        Self {
            schema_version: 1,
            theme: Theme::default(),
            slide_size: SlideSize {
                width: 1280,
                height: 720,
            },
            slides: vec![Slide::new(Background::default())],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Theme {
    pub id: String,
    pub font_heading: String,
    pub font_body: String,
    pub colors: BTreeMap<String, String>,
}

impl Default for Theme {
    fn default() -> Self {
        let colors = [
            ("primary", "#FF6F61"),
            ("primaryDark", "#E05548"),
            ("primaryLight", "#FFA396"),
        ]
        .into_iter()
        .map(|(k, v)| (k.into(), v.into()))
        .collect();
        Self {
            id: "coral-default".into(),
            font_heading: "Inter".into(),
            font_body: "Inter".into(),
            colors,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SlideSize {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Slide {
    pub id: String,
    pub background: Background,
    pub transition: Transition,
    pub notes: String,
    pub elements: Vec<Element>,
}

impl Slide {
    pub fn new(background: Background) -> Self {
        Self {
            id: id("sl"),
            background,
            transition: Transition::default(),
            notes: String::new(),
            elements: Vec::new(),
        }
    }

    pub fn duplicate(&self) -> Self {
        let mut slide = self.clone();
        slide.id = id("sl");
        for element in &mut slide.elements {
            element.id = id("el");
        }
        slide
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Background {
    Color { value: String },
    Gradient { value: String },
    Image { value: String },
}

impl Default for Background {
    fn default() -> Self {
        Self::Color {
            value: "#FFFFFF".into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Transition {
    #[serde(rename = "type")]
    pub kind: TransitionType,
    pub duration_ms: u32,
}
impl Default for Transition {
    fn default() -> Self {
        Self {
            kind: TransitionType::Fade,
            duration_ms: 300,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TransitionType {
    None,
    Fade,
    Slide,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Element {
    pub id: String,
    #[serde(flatten)]
    pub kind: ElementKind,
    pub frame: Frame,
    pub z: i32,
}

impl Element {
    pub fn text(x: f64, y: f64) -> Self {
        Self {
            id: id("el"),
            kind: ElementKind::Text {
                props: TextProps::default(),
            },
            frame: Frame {
                x,
                y,
                width: 420.0,
                height: 100.0,
                rotation: 0.0,
            },
            z: 1,
        }
    }
    pub fn shape(shape: ShapeType, x: f64, y: f64) -> Self {
        Self {
            id: id("el"),
            kind: ElementKind::Shape {
                props: ShapeProps {
                    shape,
                    fill: "#FF6F61".into(),
                    stroke: "#E05548".into(),
                    stroke_width: 2.0,
                    corner_radius: 12.0,
                },
            },
            frame: Frame {
                x,
                y,
                width: 240.0,
                height: 140.0,
                rotation: 0.0,
            },
            z: 1,
        }
    }

    pub fn image(src: String, x: f64, y: f64) -> Self {
        Self {
            id: id("el"),
            kind: ElementKind::Image {
                props: ImageProps {
                    src,
                    fit: ImageFit::Contain,
                },
            },
            frame: Frame {
                x,
                y,
                width: 480.0,
                height: 320.0,
                rotation: 0.0,
            },
            z: 1,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ElementKind {
    Text { props: TextProps },
    Image { props: ImageProps },
    Shape { props: ShapeProps },
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Frame {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub rotation: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextProps {
    pub blocks: Vec<TextBlock>,
}
impl Default for TextProps {
    fn default() -> Self {
        Self {
            blocks: vec![TextBlock::default()],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextBlock {
    pub align: TextAlign,
    pub font_family: String,
    pub font_size_pt: f64,
    pub line_height: f64,
    pub list: Option<ListType>,
    pub runs: Vec<TextRun>,
}
impl Default for TextBlock {
    fn default() -> Self {
        Self {
            align: TextAlign::Left,
            font_family: "Inter".into(),
            font_size_pt: 28.0,
            line_height: 1.2,
            list: None,
            runs: vec![TextRun {
                text: "Texto".into(),
                attrs: TextAttrs::default(),
            }],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TextAlign {
    Left,
    Center,
    Right,
    Justify,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ListType {
    Bullet,
    Ordered,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextRun {
    pub text: String,
    pub attrs: TextAttrs,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct TextAttrs {
    #[serde(default)]
    pub bold: bool,
    #[serde(default)]
    pub italic: bool,
    #[serde(default)]
    pub underline: bool,
    #[serde(default)]
    pub strikethrough: bool,
    pub color: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImageProps {
    pub src: String,
    pub fit: ImageFit,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ImageFit {
    Contain,
    Cover,
    Fill,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShapeProps {
    pub shape: ShapeType,
    pub fill: String,
    pub stroke: String,
    pub stroke_width: f64,
    pub corner_radius: f64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ShapeType {
    Rect,
    Ellipse,
    Line,
    Arrow,
}
