use super::*;

#[test]
fn curve_has_correct_endpoints() {
    let c = CubicBezier::EMPHASIZED_DECELERATE;
    assert!((c.eval(0.0)).abs() < 1e-3, "f(0) must be 0");
    assert!((c.eval(1.0) - 1.0).abs() < 1e-3, "f(1) must be 1");
}

#[test]
fn curve_is_monotonic() {
    let c = CubicBezier::EMPHASIZED_DECELERATE;
    let mut last = c.eval(0.0);
    for i in 1..=100 {
        let v = c.eval(i as f32 / 100.0);
        assert!(v >= last - 1e-4, "curve must not regress");
        last = v;
    }
}

#[test]
fn spring_settles_at_target() {
    let mut s = Spring::spatial(0.0);
    s.set_target(1.0);
    let mut steps = 0;
    while s.step(1.0 / 60.0) {
        steps += 1;
        assert!(steps < 600, "spring failed to settle");
    }
    assert!((s.value() - 1.0).abs() < 1e-3);
}

#[test]
fn settled_spring_reports_inactive() {
    let mut s = Spring::spatial(0.3);
    assert!(
        !s.step(1.0 / 60.0),
        "an idle spring must not request frames"
    );
    assert!(s.settled());
}

#[test]
fn spatial_springs_settle() {
    for mut s in [
        Spring::at(0.0, 1400.0, 0.9), // spatial fast
        Spring::at(0.0, 700.0, 0.9),  // spatial default
        Spring::at(0.0, 300.0, 0.9),  // spatial slow
    ] {
        s.set_target(1.0);
        let mut steps = 0;
        while s.step(1.0 / 60.0) {
            steps += 1;
            assert!(steps < 1200, "spatial spring failed to settle");
        }
        assert!(s.settled());
    }
}

#[test]
fn effects_springs_never_overshoot() {
    for mut s in [
        Spring::at(0.0, 3800.0, 1.0), // effects fast
        Spring::at(0.0, 1600.0, 1.0), // effects default
        Spring::at(0.0, 800.0, 1.0),  // effects slow
    ] {
        s.set_target(1.0);
        for _ in 0..900 {
            s.step(1.0 / 60.0);
            assert!(
                s.value() <= 1.0 + 1e-3,
                "critically damped spring overshot: {}",
                s.value()
            );
        }
    }
}

#[test]
fn tween_reaches_endpoints() {
    let mut t = Tween::new(0.0, 1.0, 0.2, CubicBezier::EMPHASIZED_DECELERATE);
    assert!(t.step(0.0), "tween is active before the duration elapses");
    assert!(!t.step(0.2), "tween must finish at its duration");
    assert!((t.value() - 1.0).abs() < 1e-3);
}

#[test]
fn tween_is_monotonic_for_decelerate() {
    let mut t = Tween::new(0.0, 10.0, 0.5, CubicBezier::EMPHASIZED_DECELERATE);
    let mut last = t.value();
    while t.step(1.0 / 120.0) {
        let v = t.value();
        assert!(v >= last - 1e-4, "decelerate tween must not regress");
        last = v;
    }
}
