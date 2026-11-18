use super::*;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

macro_rules! files {
    ($($path:literal),* $(,)?) => {
        const FILES: &[(&str, &[u8])] = &[
            $(($path, include_bytes!(concat!("../../../../integrations/globals/", $path)))),*
        ];
    };
}

files![
    "workflows/workflow-design.yaml",
    "agents/workflow-architect/agent.yaml",
    "agents/workflow-architect/instructions.md",
    "agents/workflow-author/agent.yaml",
    "agents/workflow-author/instructions.md",
    "agents/workflow-lead/agent.yaml",
    "agents/workflow-lead/instructions.md",
    "agents/workflow-analyst/agent.yaml",
    "agents/workflow-analyst/instructions.md",
    "agents/workflow-validator/agent.yaml",
    "agents/workflow-validator/instructions.md",
    "tasks/validate-workflow-definitions/task.yaml",
    "tasks/write-workflow-definitions/task.yaml",
    "tasks/mine-workflow-evidence/task.yaml",
    "tasks/propose-workflows/task.yaml",
    "tasks/gather-workflow-evidence/task.yaml",
    "skills/unelos-workflow-design/SKILL.md",
    "skills/unelos-workflow-design/references/building-blocks.md",
    "skills/unelos-workflow-design/references/definitions.md",
    "skills/unelos-workflow-design/references/patterns.md",
    "skills/unelos-workflow-design/references/session-mining.md",
    "skills/unelos-workflow-design/scripts/digest-sessions.py",
    "skills/unelos-workflow-design/scripts/walk-workflow.py",
];

type Manifest = BTreeMap<String, String>;

fn root() -> Result<Utf8PathBuf> {
    crate::task_document::global_root().map_err(io_error)
}

fn path(root: &Utf8Path, relative: &str) -> Utf8PathBuf {
    relative.split('/').fold(root.to_owned(), |p, s| p.join(s))
}

fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn manifest(root: &Utf8Path) -> Result<Manifest> {
    match fs::read(root.join("bundled.json")) {
        Ok(bytes) => serde_json::from_slice(&bytes).map_err(io_error),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Manifest::new()),
        Err(e) => Err(io_error(e)),
    }
}

fn read(path: &Utf8Path) -> Result<Option<Vec<u8>>> {
    match fs::read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(io_error(e)),
    }
}

pub(super) fn install() -> Result<()> {
    let root = root()?;
    let mut manifest = manifest(&root)?;
    for &(relative, bytes) in FILES {
        let target = path(&root, relative);
        let current = read(&target)?;
        let digest = hash(bytes);
        let safe = current.as_ref().is_none_or(|current| {
            current == bytes || manifest.get(relative) == Some(&hash(current))
        });
        if safe {
            global_config::atomic_write(&target, bytes)?;
            manifest.insert(relative.into(), digest);
            // Persist ownership after each write so a later failure does not lose it.
            global_config::atomic_write(
                &root.join("bundled.json"),
                &serde_json::to_vec_pretty(&manifest).map_err(io_error)?,
            )?;
            eprintln!("Bundled installed: {target}");
        } else {
            eprintln!("Bundled skipped (modified or unowned): {target}");
        }
    }
    Ok(())
}

pub(super) fn uninstall() -> Result<()> {
    let root = root()?;
    let manifest = manifest(&root)?;
    for &(relative, _) in FILES {
        let Some(digest) = manifest.get(relative) else {
            continue;
        };
        let target = path(&root, relative);
        if let Some(bytes) = read(&target)? {
            if hash(&bytes) != *digest {
                eprintln!("Bundled kept (modified): {target}");
                continue;
            }
            fs::remove_file(&target).map_err(io_error)?;
            eprintln!("Bundled removed: {target}");
        }
        let mut parent = target.parent();
        while let Some(dir) = parent.filter(|dir| *dir != root) {
            match fs::remove_dir(dir) {
                Ok(()) => parent = dir.parent(),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => parent = dir.parent(),
                Err(e) if e.kind() == std::io::ErrorKind::DirectoryNotEmpty => break,
                Err(e) => return Err(io_error(e)),
            }
        }
    }
    match fs::remove_file(root.join("bundled.json")) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(io_error(e)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_globals_match_source_tree() {
        fn collect(root: &Utf8Path, dir: &Utf8Path, files: &mut Vec<Utf8PathBuf>) {
            for entry in fs::read_dir(dir).unwrap() {
                let entry = entry.unwrap();
                let path = Utf8PathBuf::from_path_buf(entry.path()).unwrap();
                if path.is_dir() {
                    collect(root, &path, files);
                } else {
                    files.push(path.strip_prefix(root).unwrap().to_owned());
                }
            }
        }
        let root = Utf8Path::new(env!("CARGO_MANIFEST_DIR")).join("../../integrations/globals");
        let mut actual = Vec::new();
        collect(&root, &root, &mut actual);
        let mut embedded: Vec<_> = FILES
            .iter()
            .map(|(p, _)| path(Utf8Path::new(""), p))
            .collect();
        actual.sort();
        embedded.sort();
        assert_eq!(actual, embedded);
    }
}
