//! Glyph coverage compositing.
#[allow(clippy::too_many_arguments)]
pub(super) fn blend_coverage(
    target: &mut [u8],
    target_w: u32,
    target_h: u32,
    gx: i32,
    gy: i32,
    gw: i32,
    gh: i32,
    coverage: &[u8],
    color: [f32; 4],
) {
    let (sr, sg, sb, sa0) = (color[0], color[1], color[2], color[3]);
    for row in 0..gh {
        let ty = gy + row;
        if ty < 0 || ty >= target_h as i32 {
            continue;
        }
        for col in 0..gw {
            let tx = gx + col;
            if tx < 0 || tx >= target_w as i32 {
                continue;
            }
            let a = coverage[(row * gw + col) as usize] as f32 / 255.0;
            if a <= 0.0 {
                continue;
            }
            let sa = (sa0 * a).clamp(0.0, 1.0);
            let idx = ((ty * target_w as i32 + tx) * 4) as usize;
            let dr = target[idx] as f32 / 255.0;
            let dg = target[idx + 1] as f32 / 255.0;
            let db = target[idx + 2] as f32 / 255.0;
            let da = target[idx + 3] as f32 / 255.0;
            // premultiplied "source over"
            let out_r = sr * sa + dr * (1.0 - sa);
            let out_g = sg * sa + dg * (1.0 - sa);
            let out_b = sb * sa + db * (1.0 - sa);
            let out_a = sa + da * (1.0 - sa);
            target[idx] = (out_r.clamp(0.0, 1.0) * 255.0).round() as u8;
            target[idx + 1] = (out_g.clamp(0.0, 1.0) * 255.0).round() as u8;
            target[idx + 2] = (out_b.clamp(0.0, 1.0) * 255.0).round() as u8;
            target[idx + 3] = (out_a.clamp(0.0, 1.0) * 255.0).round() as u8;
        }
    }
}
