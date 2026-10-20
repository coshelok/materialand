//! BlueZ ObjectManager supplies dynamic adapter paths and paired devices.
use super::{command::run, BluetoothDevice, Radio};
use std::{io, time::Duration};
fn managed() -> io::Result<serde_json::Value> {
    let text = run(
        "busctl",
        &[
            "--system",
            "--json=short",
            "--timeout=2",
            "call",
            "org.bluez",
            "/",
            "org.freedesktop.DBus.ObjectManager",
            "GetManagedObjects",
        ],
        Duration::from_secs(3),
    )?;
    serde_json::from_str(&text).map_err(io::Error::other)
}
fn decode(value: &serde_json::Value) -> (Vec<(String, bool)>, Vec<BluetoothDevice>) {
    let mut adapters = Vec::new();
    let mut devices = Vec::new();
    if let Some(objects) = value
        .get("data")
        .and_then(|v| v.get(0))
        .and_then(|v| v.as_object())
    {
        for (path, interfaces) in objects {
            if let Some(props) = interfaces.get("org.bluez.Adapter1") {
                if let Some(powered) = props
                    .get("Powered")
                    .and_then(|v| v.get("data"))
                    .and_then(|v| v.as_bool())
                {
                    adapters.push((path.clone(), powered));
                }
            }
            if let Some(props) = interfaces.get("org.bluez.Device1") {
                let paired = props
                    .get("Paired")
                    .and_then(|v| v.get("data"))
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                if paired {
                    let name = props
                        .get("Alias")
                        .or_else(|| props.get("Name"))
                        .and_then(|v| v.get("data"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("Bluetooth device")
                        .to_owned();
                    let connected = props
                        .get("Connected")
                        .and_then(|v| v.get("data"))
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    devices.push(BluetoothDevice {
                        path: path.clone(),
                        name,
                        connected,
                    });
                }
            }
        }
    }
    devices.sort_by_key(|d| (!d.connected, d.name.clone()));
    (adapters, devices)
}
pub(super) fn status() -> (Radio, Vec<BluetoothDevice>) {
    let Ok(value) = managed() else {
        return (Radio::default(), vec![]);
    };
    let (adapters, devices) = decode(&value);
    if adapters.is_empty() {
        return (Radio::default(), devices);
    }
    let enabled = adapters.iter().any(|(_, on)| *on);
    let connected = devices.iter().filter(|d| d.connected).count();
    let label = if !enabled {
        "Off".into()
    } else if connected > 0 {
        format!("{connected} connected")
    } else {
        "On".into()
    };
    (
        Radio {
            available: true,
            enabled,
            label,
            ..Default::default()
        },
        devices,
    )
}
pub(super) fn set(enabled: bool) -> io::Result<()> {
    let value = managed()?;
    let (adapters, _) = decode(&value);
    if adapters.is_empty() {
        return Err(io::Error::other("No Bluetooth adapter"));
    }
    for (path, _) in adapters {
        run(
            "busctl",
            &[
                "--system",
                "--timeout=2",
                "set-property",
                "org.bluez",
                &path,
                "org.bluez.Adapter1",
                "Powered",
                "b",
                if enabled { "true" } else { "false" },
            ],
            Duration::from_secs(3),
        )?;
    }
    Ok(())
}
pub(super) fn connect(path: &str, on: bool) -> io::Result<()> {
    run(
        "busctl",
        &[
            "--system",
            "--timeout=8",
            "call",
            "org.bluez",
            path,
            "org.bluez.Device1",
            if on { "Connect" } else { "Disconnect" },
        ],
        Duration::from_secs(9),
    )
    .map(|_| ())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn adapters_and_paired_devices_are_decoded_without_assuming_hci0() {
        let value = serde_json::json!({"data":[{"/org/bluez/hci3":{"org.bluez.Adapter1":{"Powered":{"data":true}}},"/org/bluez/hci3/dev_AA":{"org.bluez.Device1":{"Alias":{"data":"Headphones"},"Paired":{"data":true},"Connected":{"data":true}}},"/org/bluez/hci3/dev_BB":{"org.bluez.Device1":{"Alias":{"data":"Unpaired"},"Paired":{"data":false}}}}]});
        let (a, d) = decode(&value);
        assert_eq!(a, [("/org/bluez/hci3".to_owned(), true)]);
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].name, "Headphones");
        assert!(d[0].connected);
    }
}
