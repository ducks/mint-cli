//! Linux Secret Service via libsecret's secret-tool. Secret material travels
//! only through private pipes, never argv, environment variables or files.
use secrecy::{ExposeSecret, SecretString};
use std::{
    io::Write,
    process::{Command, Stdio},
};

pub struct Store;
const ATTRS: [&str; 4] = ["service", "mint-cli", "account", "default"];
impl Store {
    pub fn check(&self) -> Result<(), String> {
        // A harmless item verifies the actual backend before collecting a password.
        let mut child = Command::new("secret-tool")
            .args([
                "store",
                "--label=mint-cli backend check",
                "service",
                "mint-cli",
                "account",
                "backend-check",
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|_| "install libsecret's secret-tool and enable Secret Service".to_string())?;
        let written = child
            .stdin
            .take()
            .ok_or("could not open credential pipe")?
            .write_all(b"backend-availability-check");
        let status = child.wait().map_err(|_| "Secret Service check failed")?;
        if written.is_err() || !status.success() {
            return Err("unlock a Secret Service backend (GNOME Keyring or KeePassXC); no plaintext fallback is used".into());
        }
        let cleared = Command::new("secret-tool")
            .args(["clear", "service", "mint-cli", "account", "backend-check"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map_err(|_| "Secret Service check cleanup failed")?;
        if !cleared.success() {
            return Err("Secret Service check cleanup failed".into());
        }
        Ok(())
    }
    pub fn load(&self) -> Result<SecretString, String> {
        let output = Command::new("secret-tool")
            .arg("lookup")
            .args(ATTRS)
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .output()
            .map_err(|_| "could not access Secret Service".to_string())?;
        if !output.status.success() {
            return Err(
                "no accessible saved session; unlock your keyring and run mint-cli login".into(),
            );
        }
        String::from_utf8(output.stdout)
            .map(|s| s.trim_end_matches('\n').to_owned().into())
            .map_err(|_| "invalid stored session".into())
    }
    pub fn save(&self, secret: &SecretString) -> Result<(), String> {
        let mut child = Command::new("secret-tool")
            .args(["store", "--label=mint-cli session"])
            .args(ATTRS)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|_| "could not access Secret Service; session was not saved".to_string())?;
        let written = child
            .stdin
            .take()
            .ok_or("could not open credential pipe")?
            .write_all(secret.expose_secret().as_bytes());
        let result = child
            .wait()
            .map_err(|_| "could not save session".to_string())?;
        if written.is_err() || !result.success() {
            return Err(
                "could not save session; unlock your Secret Service; no plaintext fallback is used"
                    .into(),
            );
        }
        Ok(())
    }
    pub fn delete(&self) -> Result<(), String> {
        let result = Command::new("secret-tool")
            .arg("clear")
            .args(ATTRS)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map_err(|_| "could not access Secret Service".to_string())?;
        if !result.success() {
            return Err("could not remove saved credentials; unlock your keyring".into());
        }
        Ok(())
    }
}

/// Serialize session rotation across CLI processes. This file contains no secrets.
pub fn session_lock() -> Result<std::fs::File, String> {
    use std::{
        fs::{self, OpenOptions},
        os::unix::fs::{DirBuilderExt, OpenOptionsExt},
        path::PathBuf,
    };
    let root = std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/state")))
        .ok_or("set an absolute XDG_STATE_HOME or HOME")?;
    let directory = root.join("mint-cli");
    fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(&directory)
        .map_err(|_| "could not create XDG state directory")?;
    let lock = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .open(directory.join("session.lock"))
        .map_err(|_| "could not open session lock")?;
    lock.try_lock()
        .map_err(|_| "another mint-cli command is using the session")?;
    Ok(lock)
}
