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
        shape.shape_bounds = bounds;
        shape.edge_bounds = bounds;
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
