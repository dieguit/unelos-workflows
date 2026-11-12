use camino::{Utf8Path, Utf8PathBuf};
pub use unelos_core::opencode_agents::check_agents;
use unelos_core::{AgentExport, InstallError};

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "lowercase")]
pub enum Version {
    V1,
    V2,
}

pub fn write_agents(
    root: &Utf8Path,
    version: Version,
    agents: &[AgentExport],
) -> Result<Vec<Utf8PathBuf>, InstallError> {
    use unelos_core::opencode_agents as core;
    core::write_agents(
        root,
        match version {
            Version::V1 => core::Version::V1,
            Version::V2 => core::Version::V2,
        },
        agents,
    )
}
