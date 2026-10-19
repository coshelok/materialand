//! Render -> synchronize -> queue. Only vblank completes a queued frame.
use super::*;
pub(super) fn render(app: &mut App) {
    let Some(gpu) = app.backend.gpu.as_mut() else {
        return;
    };
    if !app.backend.active || gpu.output.frame.is_pending() {
        return;
    }
    let size = Size::<i32, Physical>::from((app.state.display.size.w, app.state.display.size.h));
    if size.w <= 0 || size.h <= 0 {
        return;
    }
    app.state.validate_imports(&mut gpu.renderer);
    let elements = ui::build_ui(&mut app.state, &mut gpu.renderer, size);
    let result = match gpu.output.drm.render_frame(
        &mut gpu.renderer,
        &elements,
        smithay::backend::renderer::Color32F::BLACK,
        FrameFlags::DEFAULT,
    ) {
        Ok(result) => result,
        Err(error) => {
            tracing::warn!(?error, "DRM render failed");
            return;
        }
    };
    if result.is_empty {
        // The same content is already presented. A client requesting a frame
        // without damage must still be allowed to advance its animation.
        app.dirty = false;
        // An occluded or unchanged scene is already presented. Accept it for
        // entrance timing too, or a hidden entrance would request frames forever.
        app.state.shell.window_frame_presented();
        ui::send_frames_all(&app.state);
        let _ = app.display.flush_clients();
        return;
    }
    if result.needs_sync() {
        if let PrimaryPlaneElement::Swapchain(element) = &result.primary_element {
            if let Err(error) = element.sync.wait() {
                tracing::warn!(?error, "render fence wait failed");
                return;
            }
        }
    }
    drop(result);
    match gpu.output.drm.queue_frame(()) {
        Ok(()) => {
            gpu.output.frame.queued();
            app.dirty = false;
        }
        Err(FrameError::EmptyFrame) => {
            app.dirty = false;
            app.state.shell.window_frame_presented();
            ui::send_frames_all(&app.state);
            let _ = app.display.flush_clients();
        }
        Err(error) => tracing::warn!(?error, "DRM submission failed"),
    }
}
