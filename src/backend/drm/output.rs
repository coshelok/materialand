//! Output.
use super::*;

impl Backend {
    pub(super) fn attach_output(
        &mut self,
        state: &mut State,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let Some(pending) = self.pending.take() else {
            return Ok(());
        };
        let PendingGpu {
            path,
            dev_id,
            device,
            notifier,
            gbm,
            mut renderer,
            connector,
            crtc,
            mode,
        } = pending;

        // The output must carry the real mode before the DRM compositor starts
        // tracking it: `OutputModeSource` reads the mode of the Output.
        let info = device.get_connector(connector, false)?;
        let (name, physical) = connector_identity(&info);
        state.set_output_identity(name.clone(), physical);
        state.set_output_mode(to_output_mode(mode));

        state.advertise_formats(renderer.dmabuf_formats().iter().copied().collect());
        let allocator = GbmAllocator::new(
            gbm.clone(),
            GbmBufferFlags::RENDERING | GbmBufferFlags::SCANOUT,
        );
        let exporter = GbmFramebufferExporter::new(gbm.clone(), None);
        let formats = renderer
            .dmabuf_formats()
            .iter()
            .copied()
            .collect::<Vec<_>>();
        let mut manager: Manager = DrmOutputManager::new(
            device,
            allocator,
            exporter,
            Some(gbm.clone()),
            [Fourcc::Argb8888, Fourcc::Xrgb8888],
            formats,
        );

        // The initial commit paints black; the first real frame follows right
        // after through the normal render path.
        let mut elements: DrmOutputRenderElements<GlesRenderer, ui::UiElement> =
            DrmOutputRenderElements::new();
        elements.add_output(
            &crtc,
            smithay::backend::renderer::Color32F::BLACK,
            std::iter::empty(),
        );

        let drm_output = manager.initialize_output(
            crtc,
            mode,
            &[connector],
            OutputModeSource::from(&state.display.handle),
            None,
            &mut renderer,
            &elements,
        )?;
        tracing::info!(
            device = %path.display(),
            connector = ?connector,
            mode = ?mode.name(),
            refresh_hz = f64::from(to_output_mode(mode).refresh) / 1000.0,
            "output initialized"
        );

        let token = self
            .handle
            .insert_source(notifier, |event, _, app: &mut App| on_drm_event(app, event))?;
        self.sources.drm = Some(token);
        let night_light = color::NightLight::new(manager.device(), crtc);
        state.shell.night_light_available = night_light.is_some();
        if night_light.is_none() && state.shell.night_light_on {
            state.shell.toggle_night_light();
        }
        let backlight = backlight::Backlight::new(&path, &name);
        state.shell.brightness_available = backlight.is_some();
        state.shell.brightness_error = false;
        if let Some(value) = backlight.as_ref().and_then(|b| b.current()) {
            state.brightness = value;
        }
        self.gpu = Some(Gpu {
            night_light,
            backlight,
            path,
            dev_id,
            manager,
            renderer,
            output: OutputState {
                drm: drm_output,
                crtc,
                connector,
                mode,
                frame: FrameState::default(),
            },
        });
        Ok(())
    }
}

impl Backend {
    pub(super) fn refresh_output(&mut self, state: &mut State, device_id: u64) {
        if !self.active {
            return;
        }
        if let Some(gpu) = &self.gpu {
            if gpu.dev_id != device_id {
                return;
            }
            let current = Some((gpu.output.connector, gpu.output.crtc, gpu.output.mode));
            if pick_connector(gpu.manager.device()) == current {
                if let Ok(info) = gpu
                    .manager
                    .device()
                    .get_connector(gpu.output.connector, false)
                {
                    let (name, physical) = connector_identity(&info);
                    if state.output_identity_matches(&name, &physical) {
                        return;
                    }
                }
            }
        }
        self.drop_gpu();
        self.resume(state);
    }
}

fn connector_identity(info: &connector::Info) -> (String, smithay::output::PhysicalProperties) {
    let name = info.to_string();
    let size = physical_size(info.size());
    (
        name.clone(),
        smithay::output::PhysicalProperties {
            size: size.into(),
            subpixel: info.subpixel().into(),
            make: "Unknown".into(),
            model: name,
        },
    )
}

/// Missing or invalid millimeter sizes stay unknown; never invent a diagonal.
fn physical_size(size: Option<(u32, u32)>) -> (i32, i32) {
    match size {
        Some((w, h)) if w > 0 && h > 0 && w <= i32::MAX as u32 && h <= i32::MAX as u32 => {
            (w as i32, h as i32)
        }
        _ => (0, 0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connector_millimeters_are_preserved_without_a_fabricated_fallback() {
        assert_eq!(physical_size(Some((344, 194))), (344, 194));
        assert_eq!(physical_size(None), (0, 0));
        assert_eq!(physical_size(Some((0, 194))), (0, 0));
        assert_eq!(physical_size(Some((u32::MAX, 194))), (0, 0));
    }
}
