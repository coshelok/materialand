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
            "output initialized"
        );

        let token = self
            .handle
            .insert_source(notifier, |event, _, app: &mut App| on_drm_event(app, event))?;
        self.sources.drm = Some(token);
        self.gpu = Some(Gpu {
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
                return;
            }
        }
        self.drop_gpu();
        self.resume(state);
    }
}
