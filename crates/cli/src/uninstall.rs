use camino::Utf8Path;
use std::io::{self, BufRead, IsTerminal, Write};
use unelos_core::InstallError;

pub fn uninstall(
    root: &Utf8Path,
    yes: bool,
    json: bool,
) -> Result<serde_json::Value, InstallError> {
    let root = root.canonicalize_utf8().map_err(|error| InstallError {
        code: "invalid_path",
        message: format!("Could not resolve project {root}: {error}"),
    })?;
    if !yes {
        if json || !io::stdin().is_terminal() {
            return Err(InstallError {
                code: "uninstall_confirmation_required",
                message:
                    "Uninstall requires --yes or interactive confirmation; JSON mode requires --yes"
                        .into(),
            });
        }
        if !confirm(&root, &mut io::stdin().lock(), &mut io::stderr().lock()).map_err(|error| {
            InstallError {
                code: "uninstall_confirmation_required",
                message: format!("Could not read uninstall confirmation: {error}"),
            }
        })? {
            return Err(InstallError {
                code: "uninstall_cancelled",
                message: "Project uninstall cancelled".into(),
            });
        }
    }
    serde_json::to_value(unelos_core::uninstall_project(&root)?).map_err(|error| InstallError {
        code: "uninstall_output_failed",
        message: format!("Uninstall completed but result could not be serialized: {error}"),
    })
}

fn confirm(
    root: &Utf8Path,
    reader: &mut impl BufRead,
    writer: &mut impl Write,
) -> io::Result<bool> {
    write!(
        writer,
        "Uninstall Unelos from {root}?\nDelete .unelos (including workflows and run history) and generated Unelos agents? Your other project files and global integrations will remain.\nContinue? [y/N] "
    )?;
    writer.flush()?;
    let mut answer = String::new();
    reader.read_line(&mut answer)?;
    Ok(matches!(
        answer.trim().to_ascii_lowercase().as_str(),
        "y" | "yes"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use camino::Utf8Path;

    #[test]
    fn consent_accepts_only_explicit_yes() {
        for (input, accepted) in [
            ("y\n", true),
            ("YES\n", true),
            ("n\n", false),
            ("", false),
            ("\n", false),
        ] {
            let mut prompt = Vec::new();
            assert_eq!(
                confirm(
                    Utf8Path::new("/selected/project"),
                    &mut input.as_bytes(),
                    &mut prompt
                )
                .unwrap(),
                accepted
            );
            let prompt = String::from_utf8(prompt).unwrap();
            assert!(prompt.contains("/selected/project"));
            assert!(prompt.contains(".unelos"));
            assert!(prompt.contains("run history"));
            assert!(prompt.contains("generated Unelos agents"));
        }
    }
}
