//! Bounded commands run only on the worker, never on the compositor thread.
use std::{
    io::{self, Read},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};
pub(super) fn run(program: &str, args: &[&str], timeout: Duration) -> io::Result<String> {
    let mut child = Command::new(program)
        .args(args)
        .env("LC_ALL", "C")
        .env("NO_COLOR", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    // Reader threads drain pipes while the child is running, preventing a
    // full command output buffer from deadlocking the timeout/poll loop.
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let read = |pipe: Option<std::process::ChildStdout>| -> Vec<u8> {
        let mut out = Vec::new();
        if let Some(mut pipe) = pipe {
            let _ = pipe.read_to_end(&mut out);
        }
        out
    };
    let out = thread::spawn(move || read(stdout));
    let err = thread::spawn(move || {
        let mut out = Vec::new();
        if let Some(mut pipe) = stderr {
            let _ = pipe.read_to_end(&mut out);
        }
        out
    });
    let started = Instant::now();
    let mut timed_out = false;
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if started.elapsed() >= timeout {
            let _ = child.kill();
            timed_out = true;
            break child.wait()?;
        }
        thread::sleep(Duration::from_millis(10));
    };
    let output = out.join().unwrap_or_default();
    let error = err.join().unwrap_or_default();
    if timed_out {
        return Err(io::Error::new(
            io::ErrorKind::TimedOut,
            "system service did not respond",
        ));
    }
    if !status.success() {
        let error = String::from_utf8_lossy(&error);
        return Err(io::Error::other(if error.trim().is_empty() {
            "system service rejected the request".to_owned()
        } else {
            error.trim().chars().take(200).collect()
        }));
    }
    Ok(String::from_utf8_lossy(&output).trim().to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unresponsive_system_command_has_a_bounded_lifetime() {
        let start = Instant::now();
        let result = run("/usr/bin/sleep", &["5"], Duration::from_millis(15));
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::TimedOut);
        assert!(start.elapsed() < Duration::from_secs(1));
    }
}
