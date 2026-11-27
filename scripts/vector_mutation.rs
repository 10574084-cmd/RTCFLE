use swf::{FillStyle, Gradient, LineStyle, ShapeRecord, Twips};

fn next(seed: &mut u32) -> u32 {
    if *seed == 0 {
        *seed = 0xA341_316C;
    }
    let mut value = *seed;
    value ^= value << 13;
    value ^= value >> 17;
    value ^= value << 5;
    *seed = value;
    value
}

fn should_change(seed: &mut u32, intensity: u8) -> bool {
    next(seed) % 100 < u32::from(intensity.clamp(1, 100))
}

fn offset(seed: &mut u32, intensity: u8) -> i32 {
    let size = 1 + i32::from(intensity.clamp(1, 100)) * 3;
    let amount = (next(seed) % size as u32 + 1) as i32;
    if next(seed) & 1 == 0 { -amount } else { amount }
}

fn distort_twips(value: &mut Twips, seed: &mut u32, intensity: u8) {
    *value = Twips::new(value.get().saturating_add(offset(seed, intensity)));
}

fn distort_color(color: &mut swf::Color, seed: &mut u32, intensity: u8) -> u32 {
    if !should_change(seed, intensity) {
        return 0;
    }
    match next(seed) % 4 {
        0 => color.r ^= (next(seed) & 0xFF) as u8,
        1 => color.g ^= (next(seed) & 0xFF) as u8,
        2 => color.b ^= (next(seed) & 0xFF) as u8,
        _ => color.a ^= ((next(seed) & 0x7F) as u8).max(1),
    }
    1
}

fn distort_gradient(gradient: &mut Gradient, seed: &mut u32, intensity: u8) -> u32 {
    let mut changed = 0;
    if should_change(seed, intensity) {
        distort_twips(&mut gradient.matrix.tx, seed, intensity);
        changed += 1;
    }
    if should_change(seed, intensity) {
        distort_twips(&mut gradient.matrix.ty, seed, intensity);
        changed += 1;
    }
    for stop in &mut gradient.records {
        if should_change(seed, intensity) {
            let shift = (next(seed) % 65 + 1) as u8;
            stop.ratio = stop.ratio.wrapping_add(shift);
            changed += 1;
        }
        changed += distort_color(&mut stop.color, seed, intensity);
    }
    changed
}

fn distort_fill(fill: &mut FillStyle, seed: &mut u32, intensity: u8) -> u32 {
    match fill {
        FillStyle::Color(color) => distort_color(color, seed, intensity),
        FillStyle::LinearGradient(gradient) | FillStyle::RadialGradient(gradient) => {
            distort_gradient(gradient, seed, intensity)
        }
        FillStyle::FocalGradient { gradient, .. } => {
            distort_gradient(gradient, seed, intensity)
        }
        FillStyle::Bitmap { matrix, .. } => {
            let mut changed = 0;
            if should_change(seed, intensity) {
                distort_twips(&mut matrix.tx, seed, intensity);
                changed += 1;
            }
            if should_change(seed, intensity) {
                distort_twips(&mut matrix.ty, seed, intensity);
                changed += 1;
            }
            changed
        }
    }
}

fn distort_style_lists(
    fills: &mut [FillStyle],
    lines: &mut [LineStyle],
    seed: &mut u32,
    intensity: u8,
) -> u32 {
    let mut changed = 0;
    for fill in fills {
        changed += distort_fill(fill, seed, intensity);
    }
    for line in lines {
        if should_change(seed, intensity) {
            let width = line.width().get();
            let delta = offset(seed, intensity).abs();
            *line = line.clone().with_width(Twips::new(width.saturating_add(delta)));
            changed += 1;
        }
        let mut fill = line.fill_style().clone();
        let fill_changes = distort_fill(&mut fill, seed, intensity);
        if fill_changes > 0 {
            *line = line.clone().with_fill_style(fill);
            changed += fill_changes;
        }
    }
    changed
}

fn distort_records(records: &mut [ShapeRecord], seed: &mut u32, intensity: u8) -> u32 {
    let mut changed = 0;
    for record in records {
        match record {
            ShapeRecord::StraightEdge { delta } => {
                if should_change(seed, intensity) {
                    distort_twips(&mut delta.dx, seed, intensity);
                    changed += 1;
                }
                if should_change(seed, intensity) {
                    distort_twips(&mut delta.dy, seed, intensity);
                    changed += 1;
                }
            }
            ShapeRecord::CurvedEdge { control_delta, anchor_delta } => {
                if should_change(seed, intensity) {
                    distort_twips(&mut control_delta.dx, seed, intensity);
                    changed += 1;
                }
                if should_change(seed, intensity) {
                    distort_twips(&mut control_delta.dy, seed, intensity);
                    changed += 1;
                }
                if should_change(seed, intensity) {
                    distort_twips(&mut anchor_delta.dx, seed, intensity);
                    changed += 1;
                }
                if should_change(seed, intensity) {
                    distort_twips(&mut anchor_delta.dy, seed, intensity);
                    changed += 1;
                }
            }
            ShapeRecord::StyleChange(change) => {
                if let Some(point) = &mut change.move_to {
                    if should_change(seed, intensity) {
                        distort_twips(&mut point.x, seed, intensity);
                        changed += 1;
                    }
                    if should_change(seed, intensity) {
                        distort_twips(&mut point.y, seed, intensity);
                        changed += 1;
                    }
                }
                if let Some(styles) = &mut change.new_styles {
                    changed += distort_style_lists(
                        &mut styles.fill_styles,
                        &mut styles.line_styles,
                        seed,
                        intensity,
                    );
                }
            }
        }
    }
    changed
}

pub(crate) fn mutate_shape(shape: &mut swf::Shape, seed: &mut u32, intensity: u8) -> u32 {
    let mut changed = distort_style_lists(
        &mut shape.styles.fill_styles,
        &mut shape.styles.line_styles,
        seed,
        intensity,
    );
    changed += distort_records(&mut shape.shape, seed, intensity);
    if changed > 0 {
        let bounds = ruffle_render::shape_utils::calculate_shape_bounds(&shape.shape);
        shape.shape_bounds = shape.shape_bounds.union(&bounds);
        shape.edge_bounds = shape.edge_bounds.union(&bounds);
    }
    changed
}

pub(crate) fn mutate_morph_shape(
    shape: &mut swf::MorphShape,
    seed: &mut u32,
    intensity: u8,
) -> u32 {
    let mut changed = distort_style_lists(
        &mut shape.fill_styles,
        &mut shape.line_styles,
        seed,
        intensity,
    );
    changed += distort_records(&mut shape.shape, seed, intensity);
    if changed > 0 {
        let bounds = ruffle_render::shape_utils::calculate_shape_bounds(&shape.shape);
        shape.shape_bounds = bounds;
        shape.edge_bounds = bounds;
    }
    changed
}


fn stretch_value(value: &mut Twips, factor: i32) {
    let stretched = value
        .get()
        .saturating_mul(factor)
        .clamp(-750_000, 750_000);
    *value = Twips::new(stretched);
}

fn stretch_pair(dx: &mut Twips, dy: &mut Twips, seed: &mut u32) -> u32 {
    let axis = next(seed) % 3;
    let factor = if next(seed) % 9 == 0 {
        0
    } else {
        2 + (next(seed) % 6) as i32
    };
    if axis != 1 {
        stretch_value(dx, factor);
    }
    if axis != 0 {
        stretch_value(dy, factor);
    }
    1
}

fn glitch_color(color: &mut swf::Color, seed: &mut u32, intensity: u8) -> u32 {
    if !should_change(seed, intensity) {
        return 0;
    }
    match next(seed) % 6 {
        0 => color.a = 0,
        1 => color.r = 255 - color.r,
        2 => color.g = 255 - color.g,
        3 => color.b = 255 - color.b,
        4 => {
            color.r = color.r.wrapping_add((next(seed) & 0x7F) as u8);
            color.b = color.b.wrapping_sub((next(seed) & 0x7F) as u8);
        }
        _ => color.a = color.a.saturating_sub((next(seed) & 0x7F) as u8),
    }
    1
}

fn stretch_fill_matrix(matrix: &mut swf::Matrix, seed: &mut u32) -> u32 {
    let axis = next(seed) % 3;
    let factor = if next(seed) % 9 == 0 {
        0.18
    } else {
        0.45 + (next(seed) % 46) as f32 / 10.0
    };
    if axis != 1 {
        matrix.a = swf::Fixed16::from_f32((matrix.a.to_f32() * factor).clamp(-32.0, 32.0));
        matrix.b = swf::Fixed16::from_f32((matrix.b.to_f32() * factor).clamp(-32.0, 32.0));
    }
    if axis != 0 {
        matrix.c = swf::Fixed16::from_f32((matrix.c.to_f32() * factor).clamp(-32.0, 32.0));
        matrix.d = swf::Fixed16::from_f32((matrix.d.to_f32() * factor).clamp(-32.0, 32.0));
    }
    1
}

fn stretch_gradient(gradient: &mut Gradient, seed: &mut u32, intensity: u8) -> u32 {
    let mut changed = 0;
    if should_change(seed, intensity) {
        changed += stretch_fill_matrix(&mut gradient.matrix, seed);
    }
    if should_change(seed, intensity) {
        let shift = offset(seed, intensity).saturating_mul(80);
        gradient.matrix.tx = Twips::new(gradient.matrix.tx.get().saturating_add(shift));
        changed += 1;
    }
    if should_change(seed, intensity) {
        let shift = offset(seed, intensity).saturating_mul(80);
        gradient.matrix.ty = Twips::new(gradient.matrix.ty.get().saturating_add(shift));
        changed += 1;
    }
    for stop in &mut gradient.records {
        if should_change(seed, intensity) {
            stop.ratio = stop.ratio.wrapping_add((next(seed) % 192 + 32) as u8);
            changed += 1;
        }
        changed += glitch_color(&mut stop.color, seed, intensity);
    }
    changed
}

fn stretch_fill(fill: &mut FillStyle, seed: &mut u32, intensity: u8) -> u32 {
    match fill {
        FillStyle::Color(color) => glitch_color(color, seed, intensity),
        FillStyle::LinearGradient(gradient) | FillStyle::RadialGradient(gradient) => {
            stretch_gradient(gradient, seed, intensity)
        }
        FillStyle::FocalGradient { gradient, .. } => {
            stretch_gradient(gradient, seed, intensity)
        }
        FillStyle::Bitmap { matrix, .. } => {
            let mut changed = 0;
            if should_change(seed, intensity) {
                changed += stretch_fill_matrix(matrix, seed);
            }
            if should_change(seed, intensity) {
                let shift = offset(seed, intensity).saturating_mul(180);
                matrix.tx = Twips::new(matrix.tx.get().saturating_add(shift));
                changed += 1;
            }
            if should_change(seed, intensity) {
                let shift = offset(seed, intensity).saturating_mul(180);
                matrix.ty = Twips::new(matrix.ty.get().saturating_add(shift));
                changed += 1;
            }
            changed
        }
    }
}

fn stretch_style_lists(
    fills: &mut [FillStyle],
    lines: &mut [LineStyle],
    seed: &mut u32,
    intensity: u8,
) -> u32 {
    let mut changed = 0;
    for fill in fills {
        changed += stretch_fill(fill, seed, intensity);
    }
    for line in lines {
        if should_change(seed, intensity) {
            let width = line.width().get();
            let factor = if next(seed) % 7 == 0 { 0 } else { 2 + (next(seed) % 4) as i32 };
            let width = width.saturating_mul(factor).clamp(0, 65_535);
            *line = line.clone().with_width(Twips::new(width));
            changed += 1;
        }
        let mut fill = line.fill_style().clone();
        let fill_changes = stretch_fill(&mut fill, seed, intensity);
        if fill_changes > 0 {
            *line = line.clone().with_fill_style(fill);
            changed += fill_changes;
        }
    }
    changed
}

fn stretch_records(records: &mut [ShapeRecord], seed: &mut u32, intensity: u8) -> u32 {
    let mut changed = 0;
    for record in records {
        match record {
            ShapeRecord::StraightEdge { delta } => {
                if should_change(seed, intensity) {
                    changed += stretch_pair(&mut delta.dx, &mut delta.dy, seed);
                }
            }
            ShapeRecord::CurvedEdge { control_delta, anchor_delta } => {
                if should_change(seed, intensity) {
                    changed += stretch_pair(&mut control_delta.dx, &mut control_delta.dy, seed);
                }
                if should_change(seed, intensity) {
                    changed += stretch_pair(&mut anchor_delta.dx, &mut anchor_delta.dy, seed);
                }
            }
            ShapeRecord::StyleChange(change) => {
                if let Some(point) = &mut change.move_to {
                    if should_change(seed, intensity) {
                        let shift = offset(seed, intensity).saturating_mul(140);
                        point.x = Twips::new(point.x.get().saturating_add(shift));
                        changed += 1;
                    }
                    if should_change(seed, intensity) {
                        let shift = offset(seed, intensity).saturating_mul(140);
                        point.y = Twips::new(point.y.get().saturating_add(shift));
                        changed += 1;
                    }
                }
                if let Some(styles) = &mut change.new_styles {
                    changed += stretch_style_lists(
                        &mut styles.fill_styles,
                        &mut styles.line_styles,
                        seed,
                        intensity,
                    );
                }
            }
        }
    }
    changed
}

pub(crate) fn mutate_stretch_shape(shape: &mut swf::Shape, seed: &mut u32, intensity: u8) -> u32 {
    let mut changed = stretch_style_lists(
        &mut shape.styles.fill_styles,
        &mut shape.styles.line_styles,
        seed,
        intensity,
    );
    changed += stretch_records(&mut shape.shape, seed, intensity);
    if changed > 0 {
        let bounds = ruffle_render::shape_utils::calculate_shape_bounds(&shape.shape);
        shape.shape_bounds = shape.shape_bounds.union(&bounds);
        shape.edge_bounds = shape.edge_bounds.union(&bounds);
    }
    changed
}

pub(crate) fn mutate_stretch_morph_shape(
    shape: &mut swf::MorphShape,
    seed: &mut u32,
    intensity: u8,
) -> u32 {
    let mut changed = stretch_style_lists(
        &mut shape.fill_styles,
        &mut shape.line_styles,
        seed,
        intensity,
    );
    changed += stretch_records(&mut shape.shape, seed, intensity);
    if changed > 0 {
        let bounds = ruffle_render::shape_utils::calculate_shape_bounds(&shape.shape);
        shape.shape_bounds = bounds;
        shape.edge_bounds = bounds;
    }
    changed
}


/// Randomly changes the aspect ratio of a live display-object instance.
///
/// The x and y basis vectors are scaled independently, keeping the object's
/// position and rotation while producing the familiar wide/tall sprite warp.
/// A second probability controls extra shear, flips, offsets, or near-collapse.
pub(crate) fn mutate_sprite_aspect_matrix(
    matrix: &mut ruffle_render::matrix::Matrix,
    seed: &mut u32,
    intensity: u8,
    glitch_chance: u8,
) -> bool {
    let intensity = intensity.clamp(1, 100);
    let selection_chance = 28 + (u32::from(intensity) * 58 / 100);
    if next(seed) % 100 >= selection_chance {
        return false;
    }

    fn factor(seed: &mut u32, intensity: u8) -> f32 {
        let base = match next(seed) % 9 {
            0 => 0.08,
            1 => 0.18,
            2 => 0.32,
            3 => 0.55,
            4 => 0.78,
            5 => 1.35,
            6 => 1.9,
            7 => 2.7,
            _ => 4.0,
        };
        let amount = 0.30 + f32::from(intensity) / 100.0 * 0.70;
        (1.0 + (base - 1.0) * amount).clamp(0.025, 5.0)
    }

    let mut scale_x = factor(seed, intensity);
    let mut scale_y = factor(seed, intensity);

    // Prefer a visibly different ratio instead of accidentally scaling both axes equally.
    if (scale_x - scale_y).abs() < 0.38 {
        if next(seed) & 1 == 0 {
            scale_x = (scale_x * 2.0).clamp(0.025, 5.0);
        } else {
            scale_y = (scale_y * 2.0).clamp(0.025, 5.0);
        }
    }

    // Matrix columns are the transformed local X and Y axes.
    matrix.a *= scale_x;
    matrix.b *= scale_x;
    matrix.c *= scale_y;
    matrix.d *= scale_y;

    if next(seed) % 100 < u32::from(glitch_chance.min(100)) {
        let shear = 0.15 + (next(seed) % 90) as f32 / 100.0;
        let direction = if next(seed) & 1 == 0 { -1.0 } else { 1.0 };
        match next(seed) % 6 {
            0 => matrix.a = -matrix.a,
            1 => matrix.d = -matrix.d,
            2 => matrix.c += matrix.a * shear * direction,
            3 => matrix.b += matrix.d * shear * direction,
            4 => {
                let delta = ((next(seed) % 5000) as i32 - 2500) * 2;
                matrix.tx = Twips::new(matrix.tx.get().saturating_add(delta));
            }
            _ => {
                if next(seed) & 1 == 0 {
                    matrix.a *= 0.025;
                    matrix.b *= 0.025;
                } else {
                    matrix.c *= 0.025;
                    matrix.d *= 0.025;
                }
            }
        }
    }

    matrix.a = matrix.a.clamp(-20.0, 20.0);
    matrix.b = matrix.b.clamp(-20.0, 20.0);
    matrix.c = matrix.c.clamp(-20.0, 20.0);
    matrix.d = matrix.d.clamp(-20.0, 20.0);
    true
}


// Twenty dedicated vector-only corruption passes. Modes 0-19 are selectable;
// mode 20 runs all twenty sequentially using one deterministic random stream.
fn effect_pair(
    dx: &mut Twips,
    dy: &mut Twips,
    seed: &mut u32,
    intensity: u8,
    mode: u8,
    index: u32,
) -> u32 {
    let x = dx.get();
    let y = dy.get();
    let amp = i32::from(intensity.clamp(1, 100));
    match mode {
        // 0 — Shatter Lines: kick edge segments into sharp, broken directions.
        0 => {
            *dx = Twips::new(x.saturating_add(offset(seed, intensity).saturating_mul(8 + amp / 5)));
            *dy = Twips::new(y.saturating_add(offset(seed, intensity).saturating_mul(8 + amp / 5)));
        }
        // 1 — Melt Geometry: make contours sag and flow downward.
        1 => {
            let flow = offset(seed, intensity).abs().saturating_mul(2 + amp / 18);
            *dx = Twips::new(x.saturating_add(offset(seed, intensity)));
            *dy = Twips::new(y.saturating_add(flow));
        }
        // 2 — Ripple Distortion: alternating waves travel across edge records.
        2 => {
            let wave = offset(seed, intensity).saturating_mul(2 + amp / 14);
            let direction = if index % 2 == 0 { 1 } else { -1 };
            *dy = Twips::new(y.saturating_add(wave.saturating_mul(direction)));
            *dx = Twips::new(x.saturating_add(wave / 3));
        }
        // 3 — Mirror Flip: reflect selected vector axes.
        3 => {
            if next(seed) & 1 == 0 { *dx = Twips::new(x.saturating_neg()); }
            else { *dy = Twips::new(y.saturating_neg()); }
        }
        // 4 — Pixel Grid: quantize vector deltas into chunky steps.
        4 => {
            let step = 24 + amp * 9;
            let snap = |value: i32| -> i32 {
                let rounded = value.saturating_add(if value >= 0 { step / 2 } else { -(step / 2) });
                (rounded / step).saturating_mul(step)
            };
            *dx = Twips::new(snap(x));
            *dy = Twips::new(snap(y));
        }
        // 5 — Spiral Twist: rotate edge vectors around their local origin.
        5 => {
            let angle = ((next(seed) % 121) as f32 / 100.0 - 0.60)
                * (0.35 + f32::from(intensity) / 100.0);
            let (sin, cos) = angle.sin_cos();
            let nx = (x as f32 * cos - y as f32 * sin) as i32;
            let ny = (x as f32 * sin + y as f32 * cos) as i32;
            *dx = Twips::new(nx.clamp(-750_000, 750_000));
            *dy = Twips::new(ny.clamp(-750_000, 750_000));
        }
        // 6 — Outline Explosion is handled in line-style data.
        6 => return 0,
        // 7 — Gradient Storm is handled in fill-style data.
        7 => return 0,
        // 8 — Ghost Contours: offset alternating vector segments like doubled outlines.
        8 => {
            if index % 2 == 0 {
                let ghost = offset(seed, intensity).saturating_mul(2 + amp / 25);
                *dx = Twips::new(x.saturating_add(ghost));
                *dy = Twips::new(y.saturating_sub(ghost));
            }
        }
        // 9 — Vector Chaos: mix flips, bursts, and axis stretching.
        9 => match next(seed) % 4 {
            0 => { *dx = Twips::new(x.saturating_neg()); }
            1 => { *dy = Twips::new(y.saturating_neg()); }
            2 => { stretch_pair(dx, dy, seed); }
            _ => {
                *dx = Twips::new(x.saturating_add(offset(seed, intensity).saturating_mul(12)));
                *dy = Twips::new(y.saturating_add(offset(seed, intensity).saturating_mul(12)));
            }
        },
        // 10 — Angle Snap: align each edge to the nearest 45-degree direction.
        10 => {
            let length = ((x as f32 * x as f32) + (y as f32 * y as f32)).sqrt();
            if length > 0.0 {
                let step = std::f32::consts::FRAC_PI_4;
                let angle = (y as f32).atan2(x as f32);
                let snapped = (angle / step).round() * step;
                *dx = Twips::new((length * snapped.cos()).clamp(-750_000.0, 750_000.0) as i32);
                *dy = Twips::new((length * snapped.sin()).clamp(-750_000.0, 750_000.0) as i32);
            }
        }
        // 11 — Edge Pinch: compress selected segments toward their start point.
        11 => {
            let factor = 0.20 + (next(seed) % 61) as f32 / 100.0;
            *dx = Twips::new((x as f32 * factor) as i32);
            *dy = Twips::new((y as f32 * factor) as i32);
        }
        // 12 — Axis Shear: skew horizontal and vertical edge components.
        12 => {
            let shear = offset(seed, intensity) as f32 / (120.0 + amp as f32 * 3.0);
            let nx = (x as f32 + y as f32 * shear).clamp(-750_000.0, 750_000.0) as i32;
            let ny = (y as f32 - x as f32 * shear * 0.45).clamp(-750_000.0, 750_000.0) as i32;
            *dx = Twips::new(nx);
            *dy = Twips::new(ny);
        }
        // 13 — Segment Tangle: cross-mix the axes on alternating edge records.
        13 => {
            if index % 2 == 0 {
                *dx = Twips::new(y.saturating_add(offset(seed, intensity).saturating_mul(2)));
                *dy = Twips::new(x.saturating_sub(offset(seed, intensity).saturating_mul(2)));
            } else {
                *dx = Twips::new(x.saturating_neg().saturating_add(offset(seed, intensity)));
                *dy = Twips::new(y.saturating_neg().saturating_sub(offset(seed, intensity)));
            }
        }
        // 14 — Stroke Pulse is handled in line-style data.
        // 15 — Fill Bleed is handled in fill-style data.
        // 16 — Gradient Inversion is handled in gradient data.
        // 17 — Bitmap Refraction is handled in bitmap-fill matrices.
        14..=17 => return 0,
        // 18 — Contour Drift is handled on move-to records.
        18 => return 0,
        // 19 — Microfracture adds small, alternating spikes to edge segments.
        19 => {
            let spike = offset(seed, intensity).saturating_mul(1 + amp / 5);
            if index % 2 == 0 {
                *dx = Twips::new(x.saturating_add(spike));
            } else {
                *dy = Twips::new(y.saturating_sub(spike));
            }
        }
        _ => return 0,
    }
    1
}

fn mutate_fill_bleed(fill: &mut FillStyle, seed: &mut u32, intensity: u8) -> u32 {
    let bleed = |color: &mut swf::Color, seed: &mut u32| -> u32 {
        if !should_change(seed, intensity) { return 0; }
        let amount = (1 + next(seed) % 96) as u8;
        color.a = if next(seed) & 1 == 0 {
            color.a.saturating_sub(amount)
        } else {
            color.a.saturating_add(amount)
        };
        1
    };
    match fill {
        FillStyle::Color(color) => bleed(color, seed),
        FillStyle::LinearGradient(gradient)
        | FillStyle::RadialGradient(gradient)
        | FillStyle::FocalGradient { gradient, .. } => {
            let mut changed = 0;
            for stop in &mut gradient.records {
                changed += bleed(&mut stop.color, seed);
            }
            changed
        }
        FillStyle::Bitmap { matrix, .. } => {
            if should_change(seed, intensity) {
                matrix.tx = Twips::new(matrix.tx.get().saturating_add(offset(seed, intensity) * 90));
                1
            } else { 0 }
        }
    }
}

fn invert_gradient_fill(fill: &mut FillStyle) -> u32 {
    let invert = |gradient: &mut Gradient| -> u32 {
        if gradient.records.is_empty() { return 0; }
        gradient.records.reverse();
        for stop in &mut gradient.records {
            stop.ratio = u8::MAX - stop.ratio;
        }
        gradient.records.len() as u32 + 1
    };
    match fill {
        FillStyle::LinearGradient(gradient)
        | FillStyle::RadialGradient(gradient)
        | FillStyle::FocalGradient { gradient, .. } => invert(gradient),
        _ => 0,
    }
}

fn refract_bitmap_fill(fill: &mut FillStyle, seed: &mut u32, intensity: u8) -> u32 {
    if let FillStyle::Bitmap { matrix, .. } = fill {
        if should_change(seed, intensity) {
            let shear = offset(seed, intensity) as f32 / 90.0;
            let scale = 0.35 + (next(seed) % 180) as f32 / 100.0;
            matrix.a = (matrix.a * scale).clamp(-1_000_000.0, 1_000_000.0);
            matrix.d = (matrix.d / scale.max(0.1)).clamp(-1_000_000.0, 1_000_000.0);
            matrix.b = (matrix.b + shear).clamp(-1_000_000.0, 1_000_000.0);
            matrix.c = (matrix.c - shear).clamp(-1_000_000.0, 1_000_000.0);
            matrix.tx = Twips::new(matrix.tx.get().saturating_add(offset(seed, intensity) * 45));
            matrix.ty = Twips::new(matrix.ty.get().saturating_sub(offset(seed, intensity) * 45));
            return 1;
        }
    }
    0
}

fn effect_styles(
    fills: &mut [FillStyle],
    lines: &mut [LineStyle],
    seed: &mut u32,
    intensity: u8,
    mode: u8,
) -> u32 {
    let mut changed = 0;
    match mode {
        // 6 — Outline Explosion: thicken, collapse, and recolor vector strokes.
        6 => {
            for line in lines.iter_mut() {
                if should_change(seed, intensity) {
                    let width = line.width().get();
                    let factor = if next(seed) % 5 == 0 { 0 } else { 2 + (next(seed) % 7) as i32 };
                    let new_width = width.saturating_mul(factor).clamp(0, 65_535);
                    *line = line.clone().with_width(Twips::new(new_width));
                    changed += 1;
                }
                let mut fill = line.fill_style().clone();
                let before = glitch_color_count(&mut fill, seed, intensity);
                if before > 0 {
                    *line = line.clone().with_fill_style(fill);
                    changed += before;
                }
            }
        }
        // 7 — Gradient Storm: shove gradient matrices and reorder their color stops.
        7 => {
            for fill in fills.iter_mut() {
                match fill {
                    FillStyle::LinearGradient(gradient)
                    | FillStyle::RadialGradient(gradient)
                    | FillStyle::FocalGradient { gradient, .. } => {
                        changed += stretch_gradient(gradient, seed, intensity);
                        let stop_count = gradient.records.len();
                        if stop_count > 1 && should_change(seed, intensity) {
                            let shift = (next(seed) as usize) % stop_count;
                            gradient.records.rotate_left(shift);
                            changed += 1;
                        }
                    }
                    _ => {}
                }
            }
            for line in lines.iter_mut() {
                let mut fill = line.fill_style().clone();
                let before = match &mut fill {
                    FillStyle::LinearGradient(gradient)
                    | FillStyle::RadialGradient(gradient)
                    | FillStyle::FocalGradient { gradient, .. } => stretch_gradient(gradient, seed, intensity),
                    _ => 0,
                };
                if before > 0 {
                    *line = line.clone().with_fill_style(fill);
                    changed += before;
                }
            }
        }
        // 8 — Ghost Contours: leave translucent color echoes in vector fills.
        8 => {
            for fill in fills.iter_mut() {
                changed += glitch_color_count(fill, seed, intensity);
            }
            for line in lines.iter_mut() {
                let mut fill = line.fill_style().clone();
                let before = glitch_color_count(&mut fill, seed, intensity);
                if before > 0 {
                    *line = line.clone().with_fill_style(fill);
                    changed += before;
                }
            }
        }
        // 9 — Vector Chaos combines stroke and fill transformations.
        9 => changed += stretch_style_lists(fills, lines, seed, intensity),
        // 14 — Stroke Pulse: alternate narrow and oversized line widths.
        14 => {
            for (index, line) in lines.iter_mut().enumerate() {
                if should_change(seed, intensity) {
                    let width = line.width().get();
                    let pulse = match (index as u32 + next(seed)) % 4 {
                        0 => 20,
                        1 => 65,
                        2 => 180,
                        _ => 360,
                    };
                    let new_width = (i64::from(width) * pulse / 100).clamp(0, 65_535) as i32;
                    if new_width != width {
                        *line = line.clone().with_width(Twips::new(new_width));
                        changed += 1;
                    }
                }
            }
        }
        // 15 — Fill Bleed: push fill alpha values toward translucent or opaque extremes.
        15 => {
            for fill in fills.iter_mut() {
                changed += mutate_fill_bleed(fill, seed, intensity);
            }
            for line in lines.iter_mut() {
                let mut fill = line.fill_style().clone();
                let before = mutate_fill_bleed(&mut fill, seed, intensity);
                if before > 0 {
                    *line = line.clone().with_fill_style(fill);
                    changed += before;
                }
            }
        }
        // 16 — Gradient Inversion: reverse gradient stops and invert their positions.
        16 => {
            for fill in fills.iter_mut() {
                if should_change(seed, intensity) {
                    changed += invert_gradient_fill(fill);
                }
            }
            for line in lines.iter_mut() {
                if should_change(seed, intensity) {
                    let mut fill = line.fill_style().clone();
                    let before = invert_gradient_fill(&mut fill);
                    if before > 0 {
                        *line = line.clone().with_fill_style(fill);
                        changed += before;
                    }
                }
            }
        }
        // 17 — Bitmap Refraction: shear and rescale bitmap-fill coordinate matrices.
        17 => {
            for fill in fills.iter_mut() {
                changed += refract_bitmap_fill(fill, seed, intensity);
            }
            for line in lines.iter_mut() {
                let mut fill = line.fill_style().clone();
                let before = refract_bitmap_fill(&mut fill, seed, intensity);
                if before > 0 {
                    *line = line.clone().with_fill_style(fill);
                    changed += before;
                }
            }
        }
        _ => {}
    }
    changed
}

fn glitch_color_count(fill: &mut FillStyle, seed: &mut u32, intensity: u8) -> u32 {
    match fill {
        FillStyle::Color(color) => glitch_color(color, seed, intensity),
        FillStyle::LinearGradient(gradient)
        | FillStyle::RadialGradient(gradient)
        | FillStyle::FocalGradient { gradient, .. } => {
            let mut changed = 0;
            for stop in &mut gradient.records {
                changed += glitch_color(&mut stop.color, seed, intensity);
            }
            changed
        }
        FillStyle::Bitmap { matrix, .. } => {
            if should_change(seed, intensity) {
                matrix.tx = Twips::new(matrix.tx.get().saturating_add(offset(seed, intensity) * 80));
                1
            } else { 0 }
        }
    }
}

fn effect_records(records: &mut [ShapeRecord], seed: &mut u32, intensity: u8, mode: u8) -> u32 {
    let mut changed = 0;
    let mut edge_index = 0u32;
    for record in records {
        match record {
            ShapeRecord::StraightEdge { delta } => {
                if should_change(seed, intensity) {
                    changed += effect_pair(&mut delta.dx, &mut delta.dy, seed, intensity, mode, edge_index);
                }
                edge_index += 1;
            }
            ShapeRecord::CurvedEdge { control_delta, anchor_delta } => {
                if should_change(seed, intensity) {
                    changed += effect_pair(&mut control_delta.dx, &mut control_delta.dy, seed, intensity, mode, edge_index);
                }
                if should_change(seed, intensity) {
                    changed += effect_pair(&mut anchor_delta.dx, &mut anchor_delta.dy, seed, intensity, mode, edge_index.wrapping_add(1));
                }
                edge_index += 2;
            }
            ShapeRecord::StyleChange(change) => {
                if let Some(point) = &mut change.move_to {
                    if mode == 1 && should_change(seed, intensity) {
                        point.y = Twips::new(point.y.get().saturating_add(offset(seed, intensity).abs() * 3));
                        changed += 1;
                    } else if mode == 8 && should_change(seed, intensity) {
                        point.x = Twips::new(point.x.get().saturating_add(offset(seed, intensity) * 4));
                        point.y = Twips::new(point.y.get().saturating_sub(offset(seed, intensity) * 4));
                        changed += 1;
                    } else if mode == 9 && should_change(seed, intensity) {
                        point.x = Twips::new(point.x.get().saturating_add(offset(seed, intensity) * 7));
                        point.y = Twips::new(point.y.get().saturating_add(offset(seed, intensity) * 7));
                        changed += 1;
                    } else if mode == 18 && should_change(seed, intensity) {
                        let drift = offset(seed, intensity) * (2 + i32::from(intensity) / 12);
                        point.x = Twips::new(point.x.get().saturating_add(drift));
                        point.y = Twips::new(point.y.get().saturating_sub(offset(seed, intensity) * (2 + i32::from(intensity) / 12)));
                        changed += 1;
                    }
                }
                if let Some(styles) = &mut change.new_styles {
                    changed += effect_styles(&mut styles.fill_styles, &mut styles.line_styles, seed, intensity, mode);
                }
            }
        }
    }
    changed
}

fn mutate_vector_effect_shape_mode(shape: &mut swf::Shape, seed: &mut u32, intensity: u8, mode: u8) -> u32 {
    let mut changed = effect_styles(&mut shape.styles.fill_styles, &mut shape.styles.line_styles, seed, intensity, mode);
    changed += effect_records(&mut shape.shape, seed, intensity, mode);
    if changed > 0 {
        let bounds = ruffle_render::shape_utils::calculate_shape_bounds(&shape.shape);
        shape.shape_bounds = shape.shape_bounds.union(&bounds);
        shape.edge_bounds = shape.edge_bounds.union(&bounds);
    }
    changed
}

pub(crate) fn mutate_vector_effect_shape(shape: &mut swf::Shape, seed: &mut u32, intensity: u8, mode: u8) -> u32 {
    let mut changed = 0;
    if mode == 20 {
        for effect in 0..20 {
            changed += mutate_vector_effect_shape_mode(shape, seed, intensity, effect);
        }
    } else if mode < 20 {
        changed += mutate_vector_effect_shape_mode(shape, seed, intensity, mode);
    }
    changed
}

fn mutate_vector_effect_morph_mode(shape: &mut swf::MorphShape, seed: &mut u32, intensity: u8, mode: u8) -> u32 {
    let mut changed = effect_styles(&mut shape.fill_styles, &mut shape.line_styles, seed, intensity, mode);
    changed += effect_records(&mut shape.shape, seed, intensity, mode);
    if changed > 0 {
        let bounds = ruffle_render::shape_utils::calculate_shape_bounds(&shape.shape);
        shape.shape_bounds = bounds;
        shape.edge_bounds = bounds;
    }
    changed
}

pub(crate) fn mutate_vector_effect_morph_shape(shape: &mut swf::MorphShape, seed: &mut u32, intensity: u8, mode: u8) -> u32 {
    let mut changed = 0;
    if mode == 20 {
        for effect in 0..20 {
            changed += mutate_vector_effect_morph_mode(shape, seed, intensity, effect);
        }
    } else if mode < 20 {
        changed += mutate_vector_effect_morph_mode(shape, seed, intensity, mode);
    }
    changed
}


// Optional live appearance controls shared by Classic Mutation, Vector Stretch,
// and the twenty dedicated vector effects. The base corruption passes remain intact;
// when color corruption is off, only color channels are restored from a snapshot.
// Cartoon mode then posterizes fills and adds heavier vector outlines.

fn restore_color_channels(source: &swf::Color, target: &mut swf::Color) {
    target.r = source.r;
    target.g = source.g;
    target.b = source.b;
    target.a = source.a;
}

fn restore_gradient_colors(source: &Gradient, target: &mut Gradient) {
    for (old_stop, new_stop) in source.records.iter().zip(target.records.iter_mut()) {
        restore_color_channels(&old_stop.color, &mut new_stop.color);
    }
}

fn restore_fill_colors(source: &FillStyle, target: &mut FillStyle) {
    match (source, target) {
        (FillStyle::Color(old), FillStyle::Color(new)) => restore_color_channels(old, new),
        (FillStyle::LinearGradient(old), FillStyle::LinearGradient(new))
        | (FillStyle::RadialGradient(old), FillStyle::RadialGradient(new)) => {
            restore_gradient_colors(old, new);
        }
        (FillStyle::FocalGradient { gradient: old, .. }, FillStyle::FocalGradient { gradient: new, .. }) => {
            restore_gradient_colors(old, new);
        }
        _ => {}
    }
}

fn restore_style_colors(
    old_fills: &[FillStyle],
    old_lines: &[LineStyle],
    new_fills: &mut [FillStyle],
    new_lines: &mut [LineStyle],
) {
    for (old, new) in old_fills.iter().zip(new_fills.iter_mut()) {
        restore_fill_colors(old, new);
    }
    for (old, new) in old_lines.iter().zip(new_lines.iter_mut()) {
        let mut new_fill = new.fill_style().clone();
        restore_fill_colors(old.fill_style(), &mut new_fill);
        *new = new.clone().with_fill_style(new_fill);
    }
}

fn restore_record_colors(old_records: &[ShapeRecord], new_records: &mut [ShapeRecord]) {
    for (old, new) in old_records.iter().zip(new_records.iter_mut()) {
        if let (
            ShapeRecord::StyleChange(old_change),
            ShapeRecord::StyleChange(new_change),
        ) = (old, new) {
            if let (Some(old_styles), Some(new_styles)) =
                (&old_change.new_styles, &mut new_change.new_styles)
            {
                restore_style_colors(
                    &old_styles.fill_styles,
                    &old_styles.line_styles,
                    &mut new_styles.fill_styles,
                    &mut new_styles.line_styles,
                );
            }
        }
    }
}

fn cartoon_channel(value: u8, strength: u8, outline: bool) -> u8 {
    let value = if outline { (u16::from(value) * 62 / 100) as u8 } else { value };
    let levels = 3u16 + (u16::from(strength.min(100)) * 5 / 100);
    let intervals = levels - 1;
    let quantized = ((u16::from(value) * intervals + 127) / 255) * 255 / intervals;
    quantized.min(255) as u8
}

fn cartoonize_color(color: &mut swf::Color, strength: u8, outline: bool) -> u32 {
    let old = (color.r, color.g, color.b);
    color.r = cartoon_channel(color.r, strength, outline);
    color.g = cartoon_channel(color.g, strength, outline);
    color.b = cartoon_channel(color.b, strength, outline);
    if old != (color.r, color.g, color.b) { 1 } else { 0 }
}

fn cartoonize_fill(fill: &mut FillStyle, strength: u8, outline: bool) -> u32 {
    match fill {
        FillStyle::Color(color) => cartoonize_color(color, strength, outline),
        FillStyle::LinearGradient(gradient)
        | FillStyle::RadialGradient(gradient)
        | FillStyle::FocalGradient { gradient, .. } => {
            let mut changed = 0;
            for stop in &mut gradient.records {
                changed += cartoonize_color(&mut stop.color, strength, outline);
            }
            changed
        }
        FillStyle::Bitmap { .. } => 0,
    }
}

fn cartoonize_style_lists(
    fills: &mut [FillStyle],
    lines: &mut [LineStyle],
    strength: u8,
) -> u32 {
    let mut changed = 0;
    for fill in fills {
        changed += cartoonize_fill(fill, strength, false);
    }
    let width_percent = 110 + u16::from(strength.min(100)) / 2;
    for line in lines {
        let old_width = line.width().get();
        let new_width = (i64::from(old_width) * i64::from(width_percent) / 100)
            .clamp(0, 65_535) as i32;
        if new_width != old_width {
            *line = line.clone().with_width(Twips::new(new_width));
            changed += 1;
        }
        let mut fill = line.fill_style().clone();
        let fill_changed = cartoonize_fill(&mut fill, strength, true);
        if fill_changed > 0 {
            *line = line.clone().with_fill_style(fill);
            changed += fill_changed;
        }
    }
    changed
}

fn cartoonize_records(records: &mut [ShapeRecord], strength: u8) -> u32 {
    let mut changed = 0;
    for record in records {
        if let ShapeRecord::StyleChange(change) = record {
            if let Some(styles) = &mut change.new_styles {
                changed += cartoonize_style_lists(
                    &mut styles.fill_styles,
                    &mut styles.line_styles,
                    strength,
                );
            }
        }
    }
    changed
}

fn cartoonize_shape(shape: &mut swf::Shape, strength: u8) -> u32 {
    cartoonize_style_lists(
        &mut shape.styles.fill_styles,
        &mut shape.styles.line_styles,
        strength,
    ) + cartoonize_records(&mut shape.shape, strength)
}

fn cartoonize_morph_shape(shape: &mut swf::MorphShape, strength: u8) -> u32 {
    cartoonize_style_lists(&mut shape.fill_styles, &mut shape.line_styles, strength)
        + cartoonize_records(&mut shape.shape, strength)
}

fn apply_shape_appearance_options(
    before: Option<&swf::Shape>,
    shape: &mut swf::Shape,
    cartoonize: bool,
    cartoon_strength: u8,
) -> u32 {
    let mut changed = 0;
    if let Some(original) = before {
        restore_style_colors(
            &original.styles.fill_styles,
            &original.styles.line_styles,
            &mut shape.styles.fill_styles,
            &mut shape.styles.line_styles,
        );
        restore_record_colors(&original.shape, &mut shape.shape);
    }
    if cartoonize {
        changed += cartoonize_shape(shape, cartoon_strength);
    }
    changed
}

fn apply_morph_appearance_options(
    before: Option<&swf::MorphShape>,
    shape: &mut swf::MorphShape,
    cartoonize: bool,
    cartoon_strength: u8,
) -> u32 {
    let mut changed = 0;
    if let Some(original) = before {
        restore_style_colors(
            &original.fill_styles,
            &original.line_styles,
            &mut shape.fill_styles,
            &mut shape.line_styles,
        );
        restore_record_colors(&original.shape, &mut shape.shape);
    }
    if cartoonize {
        changed += cartoonize_morph_shape(shape, cartoon_strength);
    }
    changed
}

pub(crate) fn mutate_shape_with_options(
    shape: &mut swf::Shape,
    seed: &mut u32,
    intensity: u8,
    corrupt_colors: bool,
    cartoonize: bool,
    cartoon_strength: u8,
) -> u32 {
    let before = if corrupt_colors { None } else { Some(shape.clone()) };
    let mut changed = mutate_shape(shape, seed, intensity);
    changed += apply_shape_appearance_options(before.as_ref(), shape, cartoonize, cartoon_strength);
    changed
}

pub(crate) fn mutate_morph_shape_with_options(
    shape: &mut swf::MorphShape,
    seed: &mut u32,
    intensity: u8,
    corrupt_colors: bool,
    cartoonize: bool,
    cartoon_strength: u8,
) -> u32 {
    let before = if corrupt_colors { None } else { Some(shape.clone()) };
    let mut changed = mutate_morph_shape(shape, seed, intensity);
    changed += apply_morph_appearance_options(before.as_ref(), shape, cartoonize, cartoon_strength);
    changed
}

pub(crate) fn mutate_stretch_shape_with_options(
    shape: &mut swf::Shape,
    seed: &mut u32,
    intensity: u8,
    corrupt_colors: bool,
    cartoonize: bool,
    cartoon_strength: u8,
) -> u32 {
    let before = if corrupt_colors { None } else { Some(shape.clone()) };
    let mut changed = mutate_stretch_shape(shape, seed, intensity);
    changed += apply_shape_appearance_options(before.as_ref(), shape, cartoonize, cartoon_strength);
    changed
}

pub(crate) fn mutate_stretch_morph_shape_with_options(
    shape: &mut swf::MorphShape,
    seed: &mut u32,
    intensity: u8,
    corrupt_colors: bool,
    cartoonize: bool,
    cartoon_strength: u8,
) -> u32 {
    let before = if corrupt_colors { None } else { Some(shape.clone()) };
    let mut changed = mutate_stretch_morph_shape(shape, seed, intensity);
    changed += apply_morph_appearance_options(before.as_ref(), shape, cartoonize, cartoon_strength);
    changed
}

pub(crate) fn mutate_vector_effect_shape_with_options(
    shape: &mut swf::Shape,
    seed: &mut u32,
    intensity: u8,
    mode: u8,
    corrupt_colors: bool,
    cartoonize: bool,
    cartoon_strength: u8,
) -> u32 {
    let before = if corrupt_colors { None } else { Some(shape.clone()) };
    let mut changed = mutate_vector_effect_shape(shape, seed, intensity, mode);
    changed += apply_shape_appearance_options(before.as_ref(), shape, cartoonize, cartoon_strength);
    changed
}

pub(crate) fn mutate_vector_effect_morph_shape_with_options(
    shape: &mut swf::MorphShape,
    seed: &mut u32,
    intensity: u8,
    mode: u8,
    corrupt_colors: bool,
    cartoonize: bool,
    cartoon_strength: u8,
) -> u32 {
    let before = if corrupt_colors { None } else { Some(shape.clone()) };
    let mut changed = mutate_vector_effect_morph_shape(shape, seed, intensity, mode);
    changed += apply_morph_appearance_options(before.as_ref(), shape, cartoonize, cartoon_strength);
    changed
}
