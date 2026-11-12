pub(crate) mod jsonc;
mod opencode;

use crate::{
    global_config::{self, env_path, error, home, io_error, Installed},
    InstallError,
};
use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::Read,
    process::{Command, Stdio},
    time::{Duration, Instant},
};

pub type Result<T> = std::result::Result<T, InstallError>;
pub type Model = String;
pub const INTEGRATION_VERSION: &str = env!("CARGO_PKG_VERSION");
pub struct Ctx {
    pub binary: Utf8PathBuf,
}

pub trait HarnessAdapter {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn binaries(&self) -> &[&str];
    fn config_dir(&self) -> Option<Utf8PathBuf>;
    fn installable(&self) -> bool {
        false
    }
    fn version(&self, bin: &Utf8Path) -> Option<String> {
        process_output(bin, &["--version"], Duration::from_secs(2))
            .ok()
            .and_then(|s| parse_version(&s))
    }
    fn integration(&self, _cx: &Ctx) -> Result<Option<Installed>> {
        Ok(None)
    }
    fn install(&self, _cx: &Ctx) -> Result<Installed> {
        Err(error("harness_not_installable", self.id()))
    }
    fn uninstall(&self, _cx: &Ctx) -> Result<()> {
        Err(error("harness_not_installable", self.id()))
    }
    fn list_models(&self) -> Result<Vec<Model>> {
        Err(error("harness_models_unavailable", self.id()))
    }
}

struct DetectOnly {
    id: &'static str,
    name: &'static str,
    bins: &'static [&'static str],
    env: &'static str,
    default: &'static str,
}
impl HarnessAdapter for DetectOnly {
    fn id(&self) -> &str {
        self.id
    }
    fn name(&self) -> &str {
        self.name
    }
    fn binaries(&self) -> &[&str] {
        self.bins
    }
    fn config_dir(&self) -> Option<Utf8PathBuf> {
        env_path(self.env).or_else(|| home().map(|p| p.join(self.default)))
    }
}

pub fn registry() -> Vec<Box<dyn HarnessAdapter>> {
    let mut adapters: Vec<Box<dyn HarnessAdapter>> = vec![Box::new(opencode::OpenCode)];
    for (id, name, bins, env, default) in [
        (
            "claude",
            "Claude Code",
            &["claude", "claude-code"][..],
            "CLAUDE_CONFIG_DIR",
            ".claude",
        ),
        ("codex", "Codex", &["codex"][..], "CODEX_HOME", ".codex"),
        ("pi", "Pi", &["pi"][..], "PI_CODING_AGENT_DIR", ".pi/agent"),
        (
            "copilot",
            "GitHub Copilot CLI",
            &["copilot"][..],
            "COPILOT_HOME",
            ".copilot",
        ),
        (
            "cursor",
            "Cursor Agent CLI",
            &["cursor-agent", "cursor"][..],
            "CURSOR_CONFIG_DIR",
            ".cursor",
        ),
        ("droid", "Droid", &["droid"][..], "", ".factory"),
        (
            "kimi",
            "Kimi Code CLI",
            &["kimi"][..],
            "KIMI_CODE_HOME",
            ".kimi-code",
        ),
        ("qwen", "Qwen Code", &["qwen"][..], "QWEN_HOME", ".qwen"),
        ("gemini", "Gemini CLI", &["gemini"][..], "", ".gemini"),
        ("amp", "Amp", &["amp"][..], "", ".config/amp"),
        ("cline", "Cline", &["cline"][..], "", ".cline"),
    ] {
        adapters.push(Box::new(DetectOnly {
            id,
            name,
            bins,
            env,
            default,
        }));
    }
    adapters
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HarnessStatus {
    pub id: String,
    pub name: String,
    pub installable: bool,
    pub detected: bool,
    pub installed: bool,
    pub update_available: bool,
    pub version: Option<String>,
    pub integration_version: Option<String>,
    pub binary: Option<String>,
    pub config_dir: Option<String>,
}

pub(crate) fn find_binary(bins: &[&str]) -> Option<Utf8PathBuf> {
    let path = std::env::var_os("PATH")?;
    for bin in bins {
        for dir in std::env::split_paths(&path) {
            let candidate = dir.join(format!("{bin}{}", std::env::consts::EXE_SUFFIX));
            let Ok(meta) = fs::metadata(&candidate) else {
                continue;
            };
            if !meta.is_file() {
                continue;
            }
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if meta.permissions().mode() & 0o111 == 0 {
                    continue;
                }
            }
            if let Ok(path) = Utf8PathBuf::from_path_buf(candidate) {
                return Some(path);
            }
        }
    }
    None
}

pub(crate) fn parse_version(output: &str) -> Option<String> {
    output
        .split(|c: char| !c.is_ascii_alphanumeric() && !matches!(c, '.' | '-' | '+'))
        .map(|s| s.strip_prefix('v').unwrap_or(s))
        .find(|s| {
            let mut parts = s.split(['.', '-', '+']);
            (0..3).all(|_| {
                parts
                    .next()
                    .is_some_and(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))
            })
        })
        .map(str::to_owned)
}

pub(crate) fn process_output(bin: &Utf8Path, args: &[&str], timeout: Duration) -> Result<String> {
    let mut child = Command::new(bin)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(io_error)?;
    let mut stdout = child.stdout.take().unwrap();
    let reader = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        stdout
            .by_ref()
            .take(1024 * 1024)
            .read_to_end(&mut bytes)
            .map(|_| bytes)
    });
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                // Do not join: descendants may keep the pipe open beyond the deadline.
                while !reader.is_finished() && start.elapsed() < timeout {
                    std::thread::sleep(Duration::from_millis(10));
                }
                if !reader.is_finished() {
                    return Err(error("harness_timeout", bin));
                }
                let bytes = reader
                    .join()
                    .map_err(|_| error("harness_process_failed", bin))?
                    .map_err(io_error)?;
                if !status.success() {
                    return Err(error("harness_process_failed", bin));
                }
                return Ok(String::from_utf8_lossy(&bytes).into_owned());
            }
            Ok(None) if start.elapsed() < timeout => std::thread::sleep(Duration::from_millis(10)),
            result => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(match result {
                    Err(e) => io_error(e),
                    _ => error("harness_timeout", bin),
                });
            }
        }
    }
}

fn context() -> Result<Ctx> {
    Ok(Ctx {
        binary: global_config::config_dir()?
            .join(format!("bin/unelos{}", std::env::consts::EXE_SUFFIX)),
    })
}
pub fn list_harnesses() -> Vec<HarnessStatus> {
    registry()
        .iter()
        .map(|adapter| {
            let binary = find_binary(adapter.binaries());
            let config = adapter.config_dir();
            let installed = context()
                .and_then(|cx| adapter.integration(&cx))
                .ok()
                .flatten();
            HarnessStatus {
                id: adapter.id().into(),
                name: adapter.name().into(),
                installable: adapter.installable(),
                detected: binary.is_some() || config.as_ref().is_some_and(|p| p.is_dir()),
                installed: installed.is_some(),
                update_available: installed.as_ref().is_some_and(|i| {
                    version_numbers(&i.version) < version_numbers(INTEGRATION_VERSION)
                }),
                version: binary.as_ref().and_then(|b| adapter.version(b)),
                integration_version: installed.map(|i| i.version),
                binary: binary.map(String::from),
                config_dir: config.map(String::from),
            }
        })
        .collect()
}
fn version_numbers(s: &str) -> Vec<u64> {
    s.split('.')
        .take(3)
        .map(|p| p.parse().unwrap_or(0))
        .collect()
}
fn adapter(id: &str) -> Result<Box<dyn HarnessAdapter>> {
    registry()
        .into_iter()
        .find(|a| a.id() == id)
        .ok_or_else(|| error("unknown_harness", id))
}
pub fn install_harness(id: &str, cli_src: &Utf8Path) -> Result<Vec<HarnessStatus>> {
    let adapter = adapter(id)?;
    if !adapter.installable() {
        return Err(error("harness_not_installable", id));
    }
    // Validate config before provisioning or writing integration files.
    opencode::validate()?;
    let binary = provision_cli(cli_src)?;
    adapter.install(&Ctx { binary })?;
    Ok(list_harnesses())
}
pub fn uninstall_harness(id: &str) -> Result<Vec<HarnessStatus>> {
    adapter(id)?.uninstall(&context()?)?;
    Ok(list_harnesses())
}
pub fn harness_models(id: &str) -> Result<Vec<Model>> {
    adapter(id)?.list_models()
}
pub fn provision_cli(src: &Utf8Path) -> Result<Utf8PathBuf> {
    let target = context()?.binary;
    let bytes = fs::read(src).map_err(io_error)?;
    global_config::atomic_write(&target, &bytes)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&target).map_err(io_error)?.permissions();
        if perms.mode() & 0o111 != 0o111 {
            perms.set_mode(perms.mode() | 0o111);
            fs::set_permissions(&target, perms).map_err(io_error)?;
        }
    }
    Ok(target)
}

/// Remove only legacy project registrations and bundled integration assets.
pub fn migrate_project(root: &Utf8Path) -> Result<bool> {
    opencode::migrate_project(root)
}

#[cfg(test)]
mod tests;
