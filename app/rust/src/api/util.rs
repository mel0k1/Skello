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
    let deadline = Instant::now() + timeout;
    loop {
        match child.try_wait()? {
            Some(status) => {
                let out = child.wait_with_output()?;
                let text = format!(
                    "{}{}",
                    String::from_utf8_lossy(&out.stdout),
                    String::from_utf8_lossy(&out.stderr)
                );
                if status.success() {
                    return Ok(text);
                }
                anyhow::bail!("{label} failed: {text}");
            }
            None => {
                if Instant::now() > deadline {
                    let _ = child.kill();
                    anyhow::bail!("{label} timed out after {}s", timeout.as_secs());
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
