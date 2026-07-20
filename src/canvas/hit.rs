use crate::model::Frame;

pub fn point_in_frame(x: f64, y: f64, frame: &Frame) -> bool {
    let (local_x, local_y) = local_point(x, y, frame);
    (0.0..=frame.width).contains(&local_x) && (0.0..=frame.height).contains(&local_y)
}

pub fn local_point(x: f64, y: f64, frame: &Frame) -> (f64, f64) {
    let cx = frame.x + frame.width / 2.0;
    let cy = frame.y + frame.height / 2.0;
    let angle = -frame.rotation.to_radians();
    let dx = x - cx;
    let dy = y - cy;
    let local_x = dx * angle.cos() - dy * angle.sin() + frame.width / 2.0;
    let local_y = dx * angle.sin() + dy * angle.cos() + frame.height / 2.0;
    (local_x, local_y)
}
