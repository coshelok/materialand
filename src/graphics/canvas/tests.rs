use super::*;

fn px(s: &Surface, x: u32, y: u32) -> (u8, u8, u8, u8) {
    let p = s.pixmap.pixel(x, y).unwrap();
    (p.red(), p.green(), p.blue(), p.alpha())
}

#[test]
fn clear_fills_everything() {
    let mut s = Surface::new(4, 4);
    s.clear(Color::from_rgba8(10, 20, 30, 255));
    assert_eq!(px(&s, 0, 0), (10, 20, 30, 255));
    assert_eq!(px(&s, 3, 3), (10, 20, 30, 255));
}

#[test]
fn zero_radius_round_rect_fills_rect() {
    let mut s = Surface::new(10, 10);
    s.fill_round_rect(2.0, 2.0, 4.0, 4.0, 0.0, Color::from_rgba8(255, 0, 0, 255));
    assert_eq!(px(&s, 0, 0).3, 0);
    assert_eq!(px(&s, 3, 3), (255, 0, 0, 255));
}

#[test]
fn round_rect_clips_corner() {
    let mut s = Surface::new(20, 20);
    s.fill_round_rect(0.0, 0.0, 20.0, 20.0, 8.0, Color::from_rgba8(0, 0, 255, 255));
    assert!(px(&s, 0, 0).3 <= 5, "corner must be ~transparent");
    assert_eq!(px(&s, 10, 10).3, 255, "center must be opaque");
}

#[test]
fn memory_buffer_is_premultiplied() {
    let mut s = Surface::new(2, 2);
    s.clear(Color::from_rgba8(255, 0, 0, 128));
    let data = s.pixmap.data();
    assert_eq!(data[3], 128, "alpha stored as-is");
    assert!(
        (data[0] as i32 - 128).abs() <= 1,
        "premultiplied red ~= alpha"
    );
}

#[test]
fn masked_circle_erases_outside() {
    let mut s = Surface::new(20, 20);
    s.clear(Color::from_rgba8(255, 255, 255, 255));
    let masked = s.masked_circle(10.0, 10.0, 4.0);
    assert_eq!(px(&masked, 10, 10).3, 255, "inside the circle stays");
    assert_eq!(px(&masked, 0, 0).3, 0, "outside is erased");
    assert_eq!(px(&masked, 19, 10).3, 0, "outside is erased");
}

#[test]
fn cropped_masked_circle_keeps_its_offset() {
    let mut s = Surface::new(40, 40);
    s.clear(Color::from_rgba8(0, 255, 0, 255));
    let (c, (ox, oy)) = s.masked_circle_cropped(20.0, 20.0, 6.0);
    assert_eq!((ox, oy), (14, 14), "crop origin is the bbox top-left");
    assert_eq!(c.width(), 12);
    assert_eq!(c.pixmap.height(), 12);
    assert_eq!(px(&c, 6, 6), (0, 255, 0, 255), "the centre survives");
    assert_eq!(
        px(&c, 0, 0).3,
        0,
        "corners of the box are outside the circle"
    );
    // A circle larger than the surface needs no crop at all.
    let (full, off) = s.masked_circle_cropped(20.0, 20.0, 100.0);
    assert_eq!(off, (0, 0));
    assert_eq!(full.width(), 40);
    assert_eq!(px(&full, 39, 39), (0, 255, 0, 255));
    // A zero radius reveals nothing.
    let (empty, _) = s.masked_circle_cropped(5.0, 5.0, 0.0);
    assert_eq!(empty.pixmap.pixel(0, 0).map(|p| p.alpha()), Some(0));
}

#[test]
fn blit_clip_cuts_in_destination_space() {
    let mut src = Surface::new(4, 4);
    src.clear(Color::from_rgba8(255, 0, 0, 255));
    let mut dst = Surface::new(40, 40);
    // Maps the source onto dst x/y 10..18.
    let transform = SkTransform::from_scale(2.0, 2.0).post_translate(10.0, 10.0);
    // The clip stays put in destination space: only the left half survives.
    let clip = round_rect_path(10.0, 10.0, 4.0, 8.0, 1.0);
    dst.blit(&src, transform, 1.0, Some(&clip));
    assert_eq!(px(&dst, 11, 13).3, 255, "inside the clip is painted");
    assert_eq!(px(&dst, 16, 13).3, 0, "the clip cuts in device space");
}

#[test]
fn shadow_fades_outward_and_stops() {
    let mut s = Surface::new(120, 120);
    s.shadow_round_rect(40.0, 40.0, 40.0, 40.0, 4.0, 16.0, Color::BLACK);
    let near = px(&s, 60, 39).3;
    let mid = px(&s, 60, 30).3;
    let far = px(&s, 60, 27).3;
    let outside = px(&s, 60, 22).3;
    assert!(near > 0, "shadow must exist next to the surface");
    assert!(near > mid, "shadow must fade with distance");
    assert!(mid > far, "shadow must fade with distance");
    assert_eq!(outside, 0, "shadow must not extend past the spread");
    // The layers also cover the container itself; the elevated container is
    // drawn on top of them by the caller, which hides that part.
    assert!(px(&s, 60, 60).3 > 0, "the covered interior is painted too");
}

#[test]
fn clipped_fill_respects_clip_path() {
    let mut s = Surface::new(20, 20);
    let mut pb = PathBuilder::new();
    pb.push_circle(10.0, 10.0, 8.0);
    let ripple = pb.finish().unwrap();
    let clip = round_rect_path(0.0, 0.0, 12.0, 20.0, 2.0);
    s.fill_path_clipped(&ripple, &clip, Color::from_rgba8(0, 0, 255, 255));
    assert_eq!(px(&s, 8, 10).3, 255, "inside the clip");
    assert_eq!(px(&s, 15, 10).3, 0, "ripple is clipped on the right");
}
