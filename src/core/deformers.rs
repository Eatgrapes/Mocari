use super::math::{Vector2, degrees_to_radian};

pub fn rotation_deformer_transform_point(
    point: Vector2,
    angle_degrees: f32,
    scale: f32,
    translation: Vector2,
    flip_x: bool,
    flip_y: bool,
) -> Vector2 {
    let theta = degrees_to_radian(angle_degrees);
    let cos = theta.cos();
    let sin = theta.sin();
    let sign_x = if flip_x { -1.0 } else { 1.0 };
    let sign_y = if flip_y { -1.0 } else { 1.0 };

    let m00 = cos * scale * sign_x;
    let m01 = -sin * scale * sign_y;
    let m10 = sin * scale * sign_x;
    let m11 = cos * scale * sign_y;

    Vector2::new(
        m00 * point.x() + m01 * point.y() + translation.x(),
        m10 * point.x() + m11 * point.y() + translation.y(),
    )
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum WarpInterpolation {
    Quad,
    Triangle,
}

pub fn warp_deformer_transform_inside(
    local_point: Vector2,
    grid: &[Vector2],
    cols: usize,
    rows: usize,
    interpolation: WarpInterpolation,
) -> Option<Vector2> {
    if !(0.0..1.0).contains(&local_point.x()) || !(0.0..1.0).contains(&local_point.y()) {
        return None;
    }

    let stride = cols.checked_add(1)?;
    let required = stride.checked_mul(rows.checked_add(1)?)?;
    if grid.len() < required {
        return None;
    }

    let u = local_point.x() * cols as f32;
    let v = local_point.y() * rows as f32;
    let i = u.trunc() as usize;
    let j = v.trunc() as usize;
    let s = u - i as f32;
    let t = v - j as f32;

    if i >= cols || j >= rows {
        return None;
    }

    let c00 = grid[j * stride + i];
    let c10 = grid[j * stride + i + 1];
    let c01 = grid[(j + 1) * stride + i];
    let c11 = grid[(j + 1) * stride + i + 1];

    Some(match interpolation {
        WarpInterpolation::Quad => bilinear_cell(s, t, c00, c10, c01, c11),
        WarpInterpolation::Triangle => triangle_cell(s, t, c00, c10, c01, c11),
    })
}

fn bilinear_cell(
    s: f32,
    t: f32,
    c00: Vector2,
    c10: Vector2,
    c01: Vector2,
    c11: Vector2,
) -> Vector2 {
    let w00 = (1.0 - s) * (1.0 - t);
    let w10 = s * (1.0 - t);
    let w01 = (1.0 - s) * t;
    let w11 = s * t;

    Vector2::new(
        w00 * c00.x() + w10 * c10.x() + w01 * c01.x() + w11 * c11.x(),
        w00 * c00.y() + w10 * c10.y() + w01 * c01.y() + w11 * c11.y(),
    )
}

fn triangle_cell(
    s: f32,
    t: f32,
    c00: Vector2,
    c10: Vector2,
    c01: Vector2,
    c11: Vector2,
) -> Vector2 {
    if s + t <= 1.0 {
        return Vector2::new(
            c00.x() + (c10.x() - c00.x()) * s + (c01.x() - c00.x()) * t,
            c00.y() + (c10.y() - c00.y()) * s + (c01.y() - c00.y()) * t,
        );
    }

    let a = 1.0 - s;
    let b = 1.0 - t;
    Vector2::new(
        c11.x() + (c01.x() - c11.x()) * a + (c10.x() - c11.x()) * b,
        c11.y() + (c01.y() - c11.y()) * a + (c10.y() - c11.y()) * b,
    )
}
