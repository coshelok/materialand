//! Device.
use super::*;

impl Backend {
    pub fn new(handle: LoopHandle<'static, App>) -> Result<Self, Box<dyn std::error::Error>> {
        let (session, notifier) = open_session()?;
        let seat = session.seat();
        tracing::info!(seat = %seat, "acquired a libseat session");
        // The notifier stays registered for the whole process lifetime (unlike
        // the device sources it survives pause/resume), so its token is not
        // kept around.
        handle.insert_source(notifier, |event, _, app: &mut App| {
            on_session_event(app, event)
        })?;

        let mut backend = Self {
            active: true,
            session,
            seat,
            handle,
            sources: Sources::default(),
            pending: None,
            gpu: None,
        };
        backend.attach_input()?;
        backend.pending = backend.open_pending_gpu()?;
        Ok(backend)
    }

    pub(super) fn attach_input(&mut self) -> crate::Result<()> {
        let udev = UdevBackend::new(&self.seat)?;
        let mut context =
            Libinput::new_with_udev(LibinputSessionInterface::from(self.session.clone()));
        context
            .udev_assign_seat(&self.seat)
            .map_err(|_| format!("libinput cannot take seat {}", self.seat))?;
        let udev_token = self
            .handle
            .insert_source(udev, |event, _, app: &mut App| on_udev_event(app, event))?;
        match self.handle.insert_source(
            LibinputInputBackend::new(context),
            |event, _, app: &mut App| on_input_event(app, event),
        ) {
            Ok(token) => {
                self.sources.udev = Some(udev_token);
                self.sources.libinput = Some(token);
                Ok(())
            }
            Err(error) => {
                self.handle.remove(udev_token);
                Err(error.into())
            }
        }
    }

    pub(super) fn open_pending_gpu(
        &mut self,
    ) -> Result<Option<PendingGpu>, Box<dyn std::error::Error>> {
        let mut errors = Vec::new();
        let candidates = self.candidates();
        for path in candidates {
            match self.try_open_gpu(&path) {
                Ok(Some(gpu)) => return Ok(Some(gpu)),
                Ok(None) => {
                    tracing::debug!(device = %path.display(), "no connected connector");
                }
                Err(err) => {
                    tracing::debug!(device = %path.display(), %err, "cannot drive this device");
                    errors.push(format!("{}: {err}", path.display()));
                }
            }
        }
        if errors.is_empty() {
            Err("no DRM card with a connected connector was found".into())
        } else {
            Err(format!("no usable DRM device: {}", errors.join("; ")).into())
        }
    }

    pub(super) fn candidates(&self) -> Vec<PathBuf> {
        let mut list = Vec::new();
        if let Some(forced) = std::env::var_os("MATERIALAND_DRM_DEVICE") {
            list.push(PathBuf::from(forced));
        }
        if let Ok(Some(primary)) = udev::primary_gpu(&self.seat) {
            if !list.contains(&primary) {
                list.push(primary);
            }
        }
        if let Ok(all) = udev::all_gpus(&self.seat) {
            for path in all {
                if !list.contains(&path) {
                    list.push(path);
                }
            }
        }
        list
    }

    pub(super) fn try_open_gpu(
        &mut self,
        path: &Path,
    ) -> Result<Option<PendingGpu>, Box<dyn std::error::Error>> {
        let fd = self
            .session
            .open(path, OFlags::RDWR | OFlags::CLOEXEC | OFlags::NOCTTY)
            .map_err(|err| format!("cannot open the device: {err:?}"))?;

        // `DrmDeviceFd` is cloneable and shares one fd (and one DRM master)
        // between KMS and GBM; creating a second `DrmDeviceFd` from the raw fd
        // instead would drop the master lock twice.
        let drm_fd = DrmDeviceFd::new(DeviceFd::from(fd));
        let (device, notifier) = DrmDevice::new(drm_fd.clone(), false)?;

        let Some((connector, crtc, mode)) = pick_connector(&device) else {
            return Ok(None);
        };

        let gbm = GbmDevice::new(drm_fd.clone())?;
        let renderer = create_renderer(gbm.clone())?;
        let dev_id = std::fs::metadata(path)
            .map(|meta| std::os::unix::fs::MetadataExt::rdev(&meta))
            .unwrap_or_default();

        Ok(Some(PendingGpu {
            path: path.to_path_buf(),
            dev_id,
            device,
            notifier,
            gbm,
            renderer,
            connector,
            crtc,
            mode,
        }))
    }

    pub(super) fn drop_gpu(&mut self) {
        if let Some(token) = self.sources.drm.take() {
            self.handle.remove(token);
        }
        if let Some(gpu) = self.gpu.take() {
            tracing::debug!(device = %gpu.path.display(), "releasing the DRM device");
        }
        self.pending = None;
    }

    pub(super) fn resume(&mut self, state: &mut State) {
        if !self.active {
            return;
        }
        if self.gpu.is_some() || self.pending.is_some() {
            return;
        }
        if self.sources.udev.is_none() {
            if let Err(err) = self.attach_input() {
                tracing::error!(%err, "failed to reattach input devices");
                return;
            }
        }
        match self.open_pending_gpu() {
            Ok(pending) => self.pending = pending,
            Err(err) => {
                tracing::error!(%err, "failed to re-open a DRM device");
                return;
            }
        }
        if let Err(err) = self.attach_output(state) {
            tracing::error!(%err, "failed to re-initialize the output");
            self.pending = None;
            self.gpu = None;
        }
    }

    pub(super) fn pause(&mut self) {
        self.active = false;
        if let Some(token) = self.sources.udev.take() {
            self.handle.remove(token);
        }
        if let Some(token) = self.sources.libinput.take() {
            self.handle.remove(token);
        }
        self.drop_gpu();
        tracing::info!("session paused");
    }

    pub fn dmabuf_formats(&self) -> Vec<smithay::backend::allocator::Format> {
        let renderer = match self.pending.as_ref() {
            Some(pending) => &pending.renderer,
            None => match self.gpu.as_ref() {
                Some(gpu) => &gpu.renderer,
                None => return Vec::new(),
            },
        };
        renderer.dmabuf_formats().iter().copied().collect()
    }
}
