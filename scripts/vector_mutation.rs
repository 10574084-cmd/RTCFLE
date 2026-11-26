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


// Ten dedicated vector-only corruption passes. Modes 0-9 are selectable;
// mode 10 runs all ten sequentially using one deterministic random stream.
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
        _ => return 0,
    }
    1
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
            for line in lines {
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
            for fill in fills {
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
            for line in lines {
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
            for fill in fills {
                changed += glitch_color_count(fill, seed, intensity);
            }
            for line in lines {
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
    if mode == 10 {
        for effect in 0..10 {
            changed += mutate_vector_effect_shape_mode(shape, seed, intensity, effect);
        }
    } else if mode < 10 {
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
    if mode == 10 {
        for effect in 0..10 {
            changed += mutate_vector_effect_morph_mode(shape, seed, intensity, effect);
        }
    } else if mode < 10 {
        changed += mutate_vector_effect_morph_mode(shape, seed, intensity, mode);
    }
    changed
}
