//! Device events change backend lifetimes; they do not own window policy.
use super::*;
pub(super) fn on_session_event(app: &mut App, event: SessionEvent) {
    match event {
        SessionEvent::PauseSession => {
            input::cancel(&mut app.state);
            if let Some((token, _)) = app.timer.take() {
                app.backend.handle.remove(token);
            }
            app.backend.pause();
        }
        SessionEvent::ActivateSession => {
            app.backend.active = true;
            app.backend.resume(&mut app.state);
            app.request_redraw();
        }
    }
}
pub(super) fn on_udev_event(app: &mut App, event: UdevEvent) {
    if !app.backend.active {
        return;
    }
    match event {
        UdevEvent::Added { path, .. } => {
            if is_card(&path) && app.backend.gpu.is_none() {
                app.backend.resume(&mut app.state);
                app.request_redraw();
            }
        }
        UdevEvent::Changed { device_id } => {
            app.backend.refresh_output(&mut app.state, device_id);
            app.request_redraw();
        }
        UdevEvent::Removed { device_id } => {
            let current = app
                .backend
                .gpu
                .as_ref()
                .map(|gpu| gpu.dev_id)
                .or_else(|| app.backend.pending.as_ref().map(|gpu| gpu.dev_id));
            if current == Some(device_id) {
                app.backend.drop_gpu();
                app.backend.resume(&mut app.state);
                app.request_redraw();
            }
        }
    }
}
pub(super) fn on_drm_event(app: &mut App, event: DrmEvent) {
    match event {
        DrmEvent::VBlank(crtc) => on_vblank(app, crtc),
        DrmEvent::Error(error) => {
            tracing::warn!(?error, "DRM device error; reopening output");
            app.backend.drop_gpu();
            app.backend.resume(&mut app.state);
            app.request_redraw();
        }
    }
}
fn on_vblank(app: &mut App, crtc: crtc::Handle) {
    let Some(gpu) = app.backend.gpu.as_mut() else {
        return;
    };
    if gpu.output.crtc != crtc || !gpu.output.frame.complete() {
        return;
    }
    if let Err(error) = gpu.output.drm.frame_submitted() {
        tracing::warn!(?error, "DRM presentation acknowledgement failed");
    }
    ui::send_frames_all(&app.state);
    let _ = app.display.flush_clients();
    app.animating = app.ticker.tick(&mut app.state);
    app.dirty |= app.animating;
    if app.dirty {
        render(app);
    }
}
pub(super) fn on_input_event(app: &mut App, event: InputEvent<LibinputInputBackend>) {
    if !app.backend.active {
        return;
    }
    match event {
        InputEvent::Keyboard { event } => input::on_keyboard(&mut app.state, &event),
        InputEvent::PointerMotion { event } => input::on_pointer_motion_relative(
            &mut app.state,
            event.delta_x(),
            event.delta_y(),
            event.time_msec(),
        ),
        InputEvent::PointerMotionAbsolute { event } => {
            let loc = event.position_transformed(app.state.display.size);
            input::on_pointer_motion(&mut app.state, loc, event.time_msec());
        }
        InputEvent::PointerButton { event } => input::on_pointer_button(&mut app.state, &event),
        InputEvent::PointerAxis { event } => {
            input::on_pointer_axis::<LibinputInputBackend>(&mut app.state, event)
        }
        _ => return,
    }
    app.request_redraw();
}
