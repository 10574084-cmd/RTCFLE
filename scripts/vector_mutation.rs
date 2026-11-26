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
