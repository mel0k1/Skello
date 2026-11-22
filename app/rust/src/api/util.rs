use std::io::Read;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

#[flutter_rust_bridge::frb(ignore)]
pub fn run_with_timeout(
    mut cmd: Command,
    label: &str,
    timeout: Duration,
) -> anyhow::Result<String> {
    let mut child = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| anyhow::anyhow!("{label}: {e}"))?;
    // Drain both pipes from helper threads: a child writing more than
    // the pipe buffer would otherwise block on its own output, never
    // finish and trip the timeout while still doing useful work.
    let stdout_handle = child.stdout.take().map(|mut pipe| {
        std::thread::spawn(move || {
            let mut out = Vec::new();
            let _ = pipe.read_to_end(&mut out);
            out
        })
    });
    let stderr_handle = child.stderr.take().map(|mut pipe| {
        std::thread::spawn(move || {
            let mut out = Vec::new();
            let _ = pipe.read_to_end(&mut out);
            out
        })
    });
    let collect = |h: Option<std::thread::JoinHandle<Vec<u8>>>| {
        h.and_then(|h| h.join().ok())
            .map(|v| String::from_utf8_lossy(&v).into_owned())
            .unwrap_or_default()
    };
    let deadline = Instant::now() + timeout;
    loop {
        match child.try_wait()? {
            Some(status) => {
                let text = format!("{}{}", collect(stdout_handle), collect(stderr_handle));
                if status.success() {
                    return Ok(text);
                }
                anyhow::bail!("{label} failed: {text}");
            }
            None => {
                if Instant::now() > deadline {
                    let _ = child.kill();
                    let text = format!("{}{}", collect(stdout_handle), collect(stderr_handle));
                    // reap the killed process before reporting
                    let _ = child.wait();
                    anyhow::bail!("{label} timed out after {}s: {text}", timeout.as_secs());
                }
                std::thread::sleep(Duration::from_millis(100));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn captures_successful_output() {
        let mut cmd = Command::new("rustc");
        cmd.arg("--version");
        let text = run_with_timeout(cmd, "rustc", Duration::from_secs(60)).unwrap();
        assert!(text.contains("rustc"));
    }

    #[test]
    fn times_out_over_deadline() {
        let mut cmd = Command::new("rustc");
        cmd.arg("--version");
        let result = run_with_timeout(cmd, "rustc", Duration::ZERO);
        assert!(result.err().unwrap().to_string().contains("timed out"));
    }
}
