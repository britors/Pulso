use crate::model::{Background, Element, ElementKind, ShapeType, Slide, TextAlign};
use anyhow::Result;
use cairo::Context;
use gdk_pixbuf::{prelude::PixbufLoaderExt, PixbufLoader};
use gtk::gdk::prelude::GdkCairoContextExt;
use pango::{Alignment, FontDescription, SCALE};
use std::collections::HashMap;

pub fn color(value: &str) -> (f64, f64, f64, f64) {
    let s = value.trim_start_matches('#');
    if s.len() >= 6 {
        let parse = |i| u8::from_str_radix(&s[i..i + 2], 16).unwrap_or(0) as f64 / 255.0;
        (
            parse(0),
            parse(2),
            parse(4),
            if s.len() == 8 { parse(6) } else { 1.0 },
        )
    } else {
        (0.0, 0.0, 0.0, 1.0)
    }
}
fn set_color(cr: &Context, value: &str) {
    let (r, g, b, a) = color(value);
    cr.set_source_rgba(r, g, b, a);
}

pub fn render_slide(
    cr: &Context,
    slide: &Slide,
    media: Option<&HashMap<String, Vec<u8>>>,
    selection: Option<&[String]>,
) -> Result<()> {
    match &slide.background {
        Background::Color { value } => set_color(cr, value),
        Background::Gradient { .. } => set_color(cr, "#FFFFFF"),
        Background::Image { .. } => set_color(cr, "#FFFFFF"),
    }
    cr.paint()?;
    let mut elements: Vec<&Element> = slide.elements.iter().collect();
    elements.sort_by_key(|e| e.z);
    for element in elements {
        render_element(cr, element, media)?;
        if selection.is_some_and(|ids| ids.contains(&element.id)) {
            selection_frame(cr, element)?;
        }
    }
    Ok(())
}

fn render_element(
    cr: &Context,
    element: &Element,
    media: Option<&HashMap<String, Vec<u8>>>,
) -> Result<()> {
    let f = element.frame;
    cr.save()?;
    cr.translate(f.x + f.width / 2.0, f.y + f.height / 2.0);
    cr.rotate(f.rotation.to_radians());
    cr.translate(-f.width / 2.0, -f.height / 2.0);
    match &element.kind {
        ElementKind::Shape { props } => {
            set_color(cr, &props.fill);
            match props.shape {
                ShapeType::Rect => {
                    cr.rounded_rectangle(0.0, 0.0, f.width, f.height, props.corner_radius);
                    cr.fill_preserve()?;
                }
                ShapeType::Ellipse => {
                    cr.save()?;
                    cr.scale(f.width / 2.0, f.height / 2.0);
                    cr.arc(1.0, 1.0, 1.0, 0.0, std::f64::consts::TAU);
                    cr.restore()?;
                    cr.fill_preserve()?;
                }
                ShapeType::Line | ShapeType::Arrow => {
                    cr.move_to(0.0, f.height / 2.0);
                    cr.line_to(f.width, f.height / 2.0);
                }
            }
            set_color(cr, &props.stroke);
            cr.set_line_width(props.stroke_width);
            cr.stroke()?;
            if props.shape == ShapeType::Arrow {
                cr.move_to(f.width, f.height / 2.0);
                cr.line_to(f.width - 18.0, f.height / 2.0 - 10.0);
                cr.line_to(f.width - 18.0, f.height / 2.0 + 10.0);
                cr.close_path();
                cr.fill()?;
            }
        }
        ElementKind::Text { props } => {
            let layout = pangocairo::functions::create_layout(cr);
            layout.set_width((f.width * SCALE as f64) as i32);
            let mut markup = String::new();
            for (i, block) in props.blocks.iter().enumerate() {
                if i > 0 {
                    markup.push('\n');
                }
                let prefix = match block.list {
                    Some(crate::model::ListType::Bullet) => "• ",
                    Some(crate::model::ListType::Ordered) => "1. ",
                    None => "",
                };
                markup.push_str(prefix);
                for run in &block.runs {
                    let escaped = glib::markup_escape_text(&run.text);
                    let color = run.attrs.color.as_deref().unwrap_or("#202020");
                    markup.push_str(&format!(
                        "<span foreground=\"{color}\"{}{}{}{}>{escaped}</span>",
                        if run.attrs.bold {
                            " weight=\"bold\""
                        } else {
                            ""
                        },
                        if run.attrs.italic {
                            " style=\"italic\""
                        } else {
                            ""
                        },
                        if run.attrs.underline {
                            " underline=\"single\""
                        } else {
                            ""
                        },
                        if run.attrs.strikethrough {
                            " strikethrough=\"true\""
                        } else {
                            ""
                        }
                    ));
                }
                if i == 0 {
                    layout.set_alignment(match block.align {
                        TextAlign::Left => Alignment::Left,
                        TextAlign::Center => Alignment::Center,
                        TextAlign::Right => Alignment::Right,
                        TextAlign::Justify => Alignment::Left,
                    });
                    layout.set_justify(block.align == TextAlign::Justify);
                    let mut fd = FontDescription::from_string(&block.font_family);
                    fd.set_size((block.font_size_pt * SCALE as f64) as i32);
                    layout.set_font_description(Some(&fd));
                    layout.set_spacing(
                        ((block.line_height - 1.0) * block.font_size_pt * SCALE as f64) as i32,
                    );
                }
            }
            layout.set_markup(&markup);
            pangocairo::functions::show_layout(cr, &layout);
        }
        ElementKind::Image { props } => {
            let name = props.src.strip_prefix("media/").unwrap_or(&props.src);
            if let Some(bytes) = media.and_then(|items| items.get(name)) {
                let loader = PixbufLoader::new();
                loader.write(bytes)?;
                loader.close()?;
                if let Some(pixbuf) = loader.pixbuf() {
                    let pw = f64::from(pixbuf.width());
                    let ph = f64::from(pixbuf.height());
                    let (sx, sy) = match props.fit {
                        crate::model::ImageFit::Fill => (f.width / pw, f.height / ph),
                        crate::model::ImageFit::Contain => {
                            let scale = (f.width / pw).min(f.height / ph);
                            (scale, scale)
                        }
                        crate::model::ImageFit::Cover => {
                            let scale = (f.width / pw).max(f.height / ph);
                            (scale, scale)
                        }
                    };
                    cr.rectangle(0.0, 0.0, f.width, f.height);
                    cr.clip();
                    cr.translate((f.width - pw * sx) / 2.0, (f.height - ph * sy) / 2.0);
                    cr.scale(sx, sy);
                    cr.set_source_pixbuf(&pixbuf, 0.0, 0.0);
                    cr.paint()?;
                }
            } else {
                set_color(cr, "#DDDDDD");
                cr.rectangle(0.0, 0.0, f.width, f.height);
                cr.fill()?;
                set_color(cr, "#888888");
                cr.move_to(0.0, 0.0);
                cr.line_to(f.width, f.height);
                cr.move_to(f.width, 0.0);
                cr.line_to(0.0, f.height);
                cr.stroke()?;
            }
        }
    }
    cr.restore()?;
    Ok(())
}

fn selection_frame(cr: &Context, element: &Element) -> Result<()> {
    let f = element.frame;
    cr.save()?;
    cr.translate(f.x + f.width / 2.0, f.y + f.height / 2.0);
    cr.rotate(f.rotation.to_radians());
    cr.translate(-f.width / 2.0, -f.height / 2.0);
    set_color(cr, "#FF6F61");
    cr.set_line_width(2.0);
    cr.rectangle(0.0, 0.0, f.width, f.height);
    cr.stroke()?;
    for x in [0.0, f.width / 2.0, f.width] {
        for y in [0.0, f.height / 2.0, f.height] {
            if x == f.width / 2.0 && y == f.height / 2.0 {
                continue;
            }
            cr.rectangle(x - 4.0, y - 4.0, 8.0, 8.0);
            cr.fill()?;
        }
    }
    cr.move_to(f.width / 2.0, 0.0);
    cr.line_to(f.width / 2.0, -24.0);
    cr.stroke()?;
    cr.arc(f.width / 2.0, -28.0, 5.0, 0.0, std::f64::consts::TAU);
    cr.fill()?;
    cr.restore()?;
    Ok(())
}

trait RoundedRectangle {
    fn rounded_rectangle(&self, x: f64, y: f64, w: f64, h: f64, radius: f64);
}
impl RoundedRectangle for Context {
    fn rounded_rectangle(&self, x: f64, y: f64, w: f64, h: f64, radius: f64) {
        let r = radius.min(w / 2.0).min(h / 2.0);
        self.new_sub_path();
        self.arc(x + w - r, y + r, r, -std::f64::consts::FRAC_PI_2, 0.0);
        self.arc(x + w - r, y + h - r, r, 0.0, std::f64::consts::FRAC_PI_2);
        self.arc(
            x + r,
            y + h - r,
            r,
            std::f64::consts::FRAC_PI_2,
            std::f64::consts::PI,
        );
        self.arc(
            x + r,
            y + r,
            r,
            std::f64::consts::PI,
            3.0 * std::f64::consts::FRAC_PI_2,
        );
        self.close_path();
    }
}
