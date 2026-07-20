use crate::model::{Frame, STAGE_HEIGHT, STAGE_WIDTH};

pub fn transform(width: i32, height: i32, zoom: f64) -> (f64, f64, f64) {
    let fit = ((width as f64 - 40.0) / STAGE_WIDTH)
        .min((height as f64 - 40.0) / STAGE_HEIGHT)
        .max(0.05);
    let scale = (fit * zoom).clamp(0.0625, 4.0);
    (
        (width as f64 - STAGE_WIDTH * scale) / 2.0,
        (height as f64 - STAGE_HEIGHT * scale) / 2.0,
        scale,
    )
}
pub fn to_stage(x: f64, y: f64, width: i32, height: i32, zoom: f64) -> (f64, f64) {
    let (ox, oy, scale) = transform(width, height, zoom);
    ((x - ox) / scale, (y - oy) / scale)
}
#[allow(dead_code)]
pub fn snapped(mut frame: Frame, others: &[Frame]) -> Frame {
    let targets_x = [0.0, STAGE_WIDTH / 2.0, STAGE_WIDTH];
    let points_x = [frame.x, frame.x + frame.width / 2.0, frame.x + frame.width];
    for target in targets_x.into_iter().chain(
        others
            .iter()
            .flat_map(|f| [f.x, f.x + f.width / 2.0, f.x + f.width]),
    ) {
        for point in points_x {
            if (point - target).abs() <= 6.0 {
                frame.x += target - point;
                return frame;
            }
        }
    }
    frame
}
