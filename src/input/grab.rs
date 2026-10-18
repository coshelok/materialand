//! Grab.
use super::*;
pub(super) fn compute_resize(
    geom: Rectangle<i32, Logical>,
    edges: ResizeEdges,
    dx: i32,
    dy: i32,
) -> (Rectangle<i32, Logical>, (i32, i32)) {
    let min_w = 320;
    let min_h = 200;
    let mut x = geom.loc.x;
    let mut y = geom.loc.y;
    let mut w = geom.size.w;
    let mut h = geom.size.h;

    if edges.left {
        let nw = (geom.size.w - dx).max(min_w);
        x = geom.loc.x + (geom.size.w - nw);
        w = nw;
    }
    if edges.right {
        w = (geom.size.w + dx).max(min_w);
    }
    if edges.top {
        let nh = (geom.size.h - dy).max(min_h);
        y = geom.loc.y + (geom.size.h - nh);
        h = nh;
    }
    if edges.bottom {
        h = (geom.size.h + dy).max(min_h);
    }
    (Rectangle::new((x, y).into(), (w, h).into()), (w, h))
}
