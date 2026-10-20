//! NetworkManager is the authority for Wi-Fi radio state and active connection.
use super::{command::run, Radio, WifiNetwork};
use std::{io, time::Duration};
pub(super) fn radio() -> Radio {
    let enabled = match run(
        "nmcli",
        &["-t", "-f", "WIFI", "radio"],
        Duration::from_secs(2),
    ) {
        Ok(v) => match v.as_str() {
            "enabled" => true,
            "disabled" => false,
            _ => return Radio::default(),
        },
        Err(_) => return Radio::default(),
    };
    let devices = run(
        "nmcli",
        &[
            "-t",
            "-e",
            "yes",
            "-f",
            "TYPE,STATE,CONNECTION",
            "device",
            "status",
        ],
        Duration::from_secs(2),
    )
    .unwrap_or_default();
    let wifi = devices
        .lines()
        .map(fields)
        .find(|f| f.first().is_some_and(|v| v == "wifi"));
    let Some(wifi) = wifi else {
        return Radio::default();
    };
    Radio {
        available: true,
        enabled,
        label: if wifi.get(1).is_some_and(|v| v == "connected") {
            wifi.get(2).cloned().unwrap_or_else(|| "Connected".into())
        } else if enabled {
            "Not connected".into()
        } else {
            "Off".into()
        },
        ..Default::default()
    }
}
pub(super) fn networks() -> Vec<WifiNetwork> {
    let text = run(
        "nmcli",
        &[
            "-t",
            "-e",
            "yes",
            "-f",
            "SSID,SIGNAL,SECURITY,IN-USE",
            "device",
            "wifi",
            "list",
            "--rescan",
            "no",
        ],
        Duration::from_secs(2),
    )
    .unwrap_or_default();
    let mut result = Vec::<WifiNetwork>::new();
    for line in text.lines() {
        let f = fields(line);
        if f.len() != 4 || f[0].is_empty() {
            continue;
        }
        let network = WifiNetwork {
            ssid: f[0].clone(),
            signal: f[1].parse().unwrap_or(0),
            secure: !f[2].is_empty() && f[2] != "--",
            active: f[3] == "*",
        };
        if !result.iter().any(|n| n.ssid == network.ssid) {
            result.push(network);
        }
    }
    result.sort_by_key(|n| (!n.active, std::cmp::Reverse(n.signal)));
    result
}
pub(super) fn set(enabled: bool) -> io::Result<()> {
    run(
        "nmcli",
        &["radio", "wifi", if enabled { "on" } else { "off" }],
        Duration::from_secs(3),
    )
    .map(|_| ())
}
pub(super) fn connect(ssid: &str) -> io::Result<()> {
    run(
        "nmcli",
        &["--wait", "8", "device", "wifi", "connect", ssid],
        Duration::from_secs(10),
    )
    .map(|_| ())
}
pub(super) fn fields(line: &str) -> Vec<String> {
    let mut fields = vec![String::new()];
    let mut escaped = false;
    for c in line.chars() {
        if escaped {
            fields.last_mut().unwrap().push(c);
            escaped = false;
        } else if c == '\\' {
            escaped = true;
        } else if c == ':' {
            fields.push(String::new());
        } else {
            fields.last_mut().unwrap().push(c);
        }
    }
    if escaped {
        fields.last_mut().unwrap().push('\\');
    }
    fields
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn escaped_ssids_do_not_break_status_fields() {
        assert_eq!(
            fields(r"Home\:wifi\\lab:80:WPA2:*"),
            ["Home:wifi\\lab", "80", "WPA2", "*"]
        );
    }
}
