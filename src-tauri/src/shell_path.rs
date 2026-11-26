use std::{
    env,
    ffi::OsString,
    process::{Command, Stdio},
    sync::mpsc,
    time::Duration,
};

const MARK: &str = "__UNELOS_PATH__";

/// Apps launched from Finder or a desktop menu get a minimal PATH, so harness CLIs
/// installed in user dirs (~/.local/bin, ~/.opencode/bin) are invisible without the login shell's PATH.
pub fn load_login_path() {
    if cfg!(windows) {
        return;
    }
    let Some(shell_path) = login_shell_path() else {
        // Runs before the log plugin exists.
        eprintln!("Could not read the login shell PATH; harness detection uses the inherited PATH");
        return;
    };
    let current = env::var_os("PATH").unwrap_or_default();
    if let Some(path) = merge(&shell_path, &current) {
        env::set_var("PATH", path);
    }
}

fn login_shell_path() -> Option<String> {
    let shell = env::var("SHELL")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "/bin/sh".into());
    let (tx, rx) = mpsc::channel();
    // shortcut: a shell that hangs past the timeout is left running, upgrade to kill it if that shows up in practice.
    std::thread::spawn(move || {
        let output = Command::new(shell)
            .args(["-ilc", &format!("printf '{MARK}%s{MARK}' \"$PATH\"")])
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .output();
        let _ = tx.send(output);
    });
    let output = rx.recv_timeout(Duration::from_secs(5)).ok()?.ok()?;
    extract(&String::from_utf8_lossy(&output.stdout))
}

fn extract(stdout: &str) -> Option<String> {
    let (_, rest) = stdout.split_once(MARK)?;
    let (path, _) = rest.split_once(MARK)?;
    (!path.is_empty()).then(|| path.to_owned())
}

fn merge(shell_path: &str, current: &OsString) -> Option<OsString> {
    let mut dirs: Vec<_> = env::split_paths(shell_path).collect();
    for dir in env::split_paths(current) {
        if !dirs.contains(&dir) {
            dirs.push(dir);
        }
    }
    env::join_paths(dirs).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_path_between_markers_ignoring_shell_noise() {
        let out = format!("welcome!\n{MARK}/a:/b{MARK}bye");
        assert_eq!(extract(&out).as_deref(), Some("/a:/b"));
        assert_eq!(extract("no markers"), None);
        assert_eq!(extract(&format!("{MARK}{MARK}")), None);
    }

    #[test]
    fn merge_puts_shell_dirs_first_and_keeps_inherited_extras_once() {
        let merged = merge("/home/u/.local/bin:/usr/bin", &OsString::from("/usr/bin:/bin")).unwrap();
        assert_eq!(merged, OsString::from("/home/u/.local/bin:/usr/bin:/bin"));
    }

    #[cfg(unix)]
    #[test]
    fn reads_path_from_a_real_login_shell() {
        assert!(login_shell_path().is_some_and(|p| p.contains("/bin")));
    }
}
