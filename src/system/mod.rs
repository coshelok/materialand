//! System service work is isolated from rendering and never reports optimistic state.
mod battery;
mod bluetooth;
mod command;
mod network;
use std::{
    path::Path,
    sync::mpsc::{self, Receiver, SyncSender},
    thread,
    time::Duration,
};
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Radio {
    pub available: bool,
    pub enabled: bool,
    pub label: String,
    pub busy: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Battery {
    pub percent: u8,
    pub charging: bool,
    pub plugged: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BluetoothDevice {
    pub path: String,
    pub name: String,
    pub connected: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WifiNetwork {
    pub ssid: String,
    pub signal: u8,
    pub secure: bool,
    pub active: bool,
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Status {
    pub battery: Option<Battery>,
    pub wifi: Radio,
    pub bluetooth: Radio,
    pub networks: Vec<WifiNetwork>,
    pub devices: Vec<BluetoothDevice>,
    pub error: Option<String>,
}
#[derive(Clone, Debug)]
pub enum Action {
    Wifi(bool),
    Bluetooth(bool),
    Connect(String),
    BluetoothDevice(String, bool),
    Refresh,
}
#[derive(Default)]
pub struct Services {
    pub status: Status,
    tx: Option<SyncSender<(u64, Action)>>,
    rx: Option<Receiver<(u64, Status)>>,
    sequence: u64,
    pending: Option<(u64, bool)>,
}
impl Services {
    pub fn start() -> Self {
        let (tx, requests) = mpsc::sync_channel(1);
        let (updates, rx) = mpsc::sync_channel(1);
        thread::spawn(move || {
            let mut status = Status::default();
            let mut completed = 0;
            loop {
                status.battery = battery::read(Path::new("/sys/class/power_supply"));
                status.wifi = network::radio();
                let (radio, devices) = bluetooth::status();
                status.bluetooth = radio;
                status.devices = devices;
                status.networks = if status.wifi.available && status.wifi.enabled {
                    network::networks()
                } else {
                    vec![]
                };
                if updates.send((completed, status.clone())).is_err() {
                    return;
                }
                match requests.recv_timeout(Duration::from_secs(2)) {
                    Ok((sequence, action)) => {
                        let result = match action {
                            Action::Wifi(on) => network::set(on),
                            Action::Bluetooth(on) => bluetooth::set(on),
                            Action::Connect(ssid) => network::connect(&ssid),
                            Action::BluetoothDevice(path, on) => bluetooth::connect(&path, on),
                            Action::Refresh => Ok(()),
                        };
                        status.error = result.err().map(|e| e.to_string());
                        completed = sequence;
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) => {}
                    Err(mpsc::RecvTimeoutError::Disconnected) => return,
                }
            }
        });
        Self {
            status: Status::default(),
            tx: Some(tx),
            rx: Some(rx),
            sequence: 0,
            pending: None,
        }
    }
    pub fn poll(&mut self) -> bool {
        let mut changed = false;
        if let Some(rx) = &self.rx {
            while let Ok((completed, mut status)) = rx.try_recv() {
                if self
                    .pending
                    .is_some_and(|(sequence, _)| completed >= sequence)
                {
                    self.pending = None;
                }
                if let Some((_, bluetooth)) = self.pending {
                    status.wifi.busy = !bluetooth;
                    status.bluetooth.busy = bluetooth;
                    status.error = None;
                }
                changed |= self.status != status;
                self.status = status;
            }
        }
        changed
    }
    pub fn request(&mut self, action: Action) {
        if self.pending.is_some() {
            return;
        }
        let sequence = self.sequence.wrapping_add(1);
        if self
            .tx
            .as_ref()
            .is_some_and(|tx| tx.try_send((sequence, action.clone())).is_ok())
        {
            self.sequence = sequence;
            if !matches!(action, Action::Refresh) {
                let bluetooth =
                    matches!(action, Action::Bluetooth(_) | Action::BluetoothDevice(_, _));
                self.pending = Some((sequence, bluetooth));
                self.status.wifi.busy = !bluetooth;
                self.status.bluetooth.busy = bluetooth;
            }
            self.status.error = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stale_poll_cannot_acknowledge_a_radio_request_and_failure_restores_actual_state() {
        let (commands, requests) = mpsc::sync_channel(1);
        let (updates, results) = mpsc::sync_channel(1);
        let mut services = Services {
            tx: Some(commands),
            rx: Some(results),
            ..Default::default()
        };
        services.status.wifi = Radio {
            available: true,
            enabled: false,
            label: "Off".into(),
            busy: false,
        };
        services.request(Action::Wifi(true));
        let (sequence, _) = requests.try_recv().unwrap();
        assert!(services.status.wifi.busy);
        assert!(!services.status.wifi.enabled);
        updates.send((0, Status::default())).unwrap();
        services.poll();
        assert!(services.status.wifi.busy);
        let mut failed = Status {
            error: Some("Permission denied".into()),
            ..Default::default()
        };
        failed.wifi.available = true;
        updates.send((sequence, failed)).unwrap();
        services.poll();
        assert!(!services.status.wifi.busy);
        assert!(!services.status.wifi.enabled);
        assert_eq!(services.status.error.as_deref(), Some("Permission denied"));
    }
}
