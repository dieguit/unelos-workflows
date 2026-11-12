use camino::{Utf8Path, Utf8PathBuf};
use serde::Serialize;
use serde_yaml_ng::{Mapping, Value};
use std::{fs, io::Write};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillDocument {
    pub name: String,
    pub description: Option<String>,
    pub body: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveSkillResult {
    pub skill: SkillDocument,
    pub updated_references: Vec<String>,
}

pub(crate) fn valid_id(id: &str) -> Result<(), String> {
    if id.is_empty()
        || id.trim() != id
        || matches!(id, "." | "..")
        || id.contains(['/', '\\', ':', '\0'])
    {
        return Err(format!("invalid Skill ID {id:?}"));
    }
    Ok(())
}

fn bounded(root: &Utf8Path, relative: &str) -> Result<Utf8PathBuf, String> {
    crate::project::bounded_path(root, Utf8Path::new(relative), root, "skill")
        .map_err(|e| e.to_string())
}

fn split(text: &str) -> Result<(Mapping, &str), String> {
    let Some(rest) = text
        .strip_prefix("---\n")
        .or_else(|| text.strip_prefix("---\r\n"))
    else {
        return Ok((Mapping::new(), text));
    };
    let mut offset = 0;
    for line in rest.split_inclusive('\n') {
        if line.trim_end_matches(['\r', '\n']) == "---" {
            let metadata = serde_yaml_ng::from_str::<Option<Mapping>>(&rest[..offset])
                .map_err(|e| e.to_string())?
                .unwrap_or_default();
            return Ok((metadata, &rest[offset + line.len()..]));
        }
        offset += line.len();
    }
    Err("Unclosed Skill frontmatter".into())
}

pub fn get_skill(root: &Utf8Path, id: &str) -> Result<SkillDocument, String> {
    valid_id(id)?;
    let root = crate::project::absolute_root(root)
        .map_err(|e| e.to_string())?
        .join(".unelos");
    let text = fs::read_to_string(bounded(&root, &format!("skills/{id}/SKILL.md"))?)
        .map_err(|e| e.to_string())?;
    let (metadata, body) = split(&text)?;
    let description = metadata
        .get(Value::from("description"))
        .map(|v| {
            v.as_str()
                .map(str::to_owned)
                .ok_or("Skill description must be a string")
        })
        .transpose()?
        .filter(|s| !s.trim().is_empty());
    Ok(SkillDocument {
        name: id.into(),
        description,
        body: body.into(),
    })
}

fn rename_refs(value: &mut Value, old: &str, new: &str, field: &str) {
    match value {
        Value::Mapping(map) => {
            for (key, value) in map {
                if key.as_str() == Some(field) {
                    if let Value::Sequence(items) = value {
                        for item in items {
                            if item.as_str() == Some(old) {
                                *item = Value::from(new);
                            }
                        }
                    }
                } else {
                    rename_refs(value, old, new, field);
                }
            }
        }
        Value::Sequence(items) => {
            for item in items {
                rename_refs(item, old, new, field);
            }
        }
        _ => {}
    }
}

// Accept a textual edit only if its semantic change is confined to skills lists.
fn reference_edit(text: &str, old: &str, new: &str) -> Result<String, String> {
    reference_field_edit(text, old, new, "skills")
}

pub(crate) fn reference_field_edit(
    text: &str,
    old: &str,
    new: &str,
    field: &str,
) -> Result<String, String> {
    let original: Value = serde_yaml_ng::from_str(text).map_err(|e| e.to_string())?;
    let mut expected = original.clone();
    rename_refs(&mut expected, old, new, field);
    if expected == original {
        return Ok(text.into());
    }
    let replacement = serde_json::to_string(new).map_err(|e| e.to_string())?;
    let mut result = text.to_owned();
    let mut current = original;
    let bytes = text.as_bytes();
    let mut spans = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i].is_ascii_whitespace() || b",[]{}:#".contains(&bytes[i]) {
            i += 1;
            continue;
        }
        let start = i;
        if matches!(bytes[i], b'\'' | b'"') {
            let quote = bytes[i];
            i += 1;
            while i < bytes.len() {
                if quote == b'"' && bytes[i] == b'\\' {
                    i = (i + 2).min(bytes.len());
                    continue;
                }
                if bytes[i] == quote {
                    i += 1;
                    if quote == b'\'' && bytes.get(i) == Some(&quote) {
                        i += 1;
                        continue;
                    }
                    break;
                }
                i += 1;
            }
        } else {
            while i < bytes.len()
                && !bytes[i].is_ascii_whitespace()
                && !b",[]{}:#".contains(&bytes[i])
            {
                i += 1;
            }
        }
        if serde_yaml_ng::from_str::<String>(&text[start..i])
            .ok()
            .as_deref()
            == Some(old)
        {
            spans.push(start..i);
        }
    }
    spans.extend(
        text.match_indices(old)
            .map(|(start, value)| start..start + value.len()),
    );
    spans.sort_by_key(|span| (span.start, span.end));
    spans.dedup();
    // Prefer complete quoted scalars over their inner substring.
    let complete = spans.clone();
    spans.retain(|span| {
        !complete
            .iter()
            .any(|other| other != span && other.start <= span.start && other.end >= span.end)
    });
    for span in spans.into_iter().rev() {
        let mut candidate = result.clone();
        candidate.replace_range(span, &replacement);
        if let Ok(parsed) = serde_yaml_ng::from_str::<Value>(&candidate) {
            let mut normalized = parsed.clone();
            rename_refs(&mut normalized, old, new, field);
            // Normalization must leave every value outside skills unchanged.
            let mut before = current.clone();
            rename_refs(&mut before, old, new, field);
            if normalized == before && parsed != current {
                result = candidate;
                current = parsed;
            }
        }
    }
    if current != expected {
        return Err("Cannot safely rewrite Skill references in this YAML formatting".into());
    }
    Ok(result)
}

pub(crate) fn collect(
    root: &Utf8Path,
    relative: &str,
    files: &mut Vec<Utf8PathBuf>,
) -> Result<(), String> {
    let path = bounded(root, relative)?;
    if !path.exists() {
        return Ok(());
    }
    if path.is_dir() {
        for entry in fs::read_dir(path).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let name = entry
                .file_name()
                .into_string()
                .map_err(|_| "Non-UTF-8 definition path")?;
            collect(root, &format!("{relative}/{name}"), files)?;
        }
    } else if (relative.starts_with("workflows/")
        && relative.matches('/').count() == 1
        && path.extension() == Some("yaml"))
        || (relative.matches('/').count() == 2
            && matches!(path.file_name(), Some("agent.yaml" | "task.yaml")))
    {
        files.push(path);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_fs::TempDir;

    fn fixture(text: &str) -> (TempDir, Utf8PathBuf) {
        let temp = TempDir::new().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().to_owned()).unwrap();
        fs::create_dir_all(root.join(".unelos/skills/old")).unwrap();
        fs::write(root.join(".unelos/skills/old/SKILL.md"), text).unwrap();
        (temp, root)
    }

    #[test]
    fn frontmatter_round_trip_preserves_extra_keys_and_body() {
        let (_temp, root) =
            fixture("---\nname: old\ndescription: before\nextra:\n  enabled: true\n---\n\nBody\n");
        let skill = get_skill(&root, "old").unwrap();
        assert_eq!(skill.body, "\nBody\n");
        save_skill(&root, "old", "old", "after", &skill.body).unwrap();
        let text = fs::read_to_string(root.join(".unelos/skills/old/SKILL.md")).unwrap();
        let (metadata, body) = split(&text).unwrap();
        assert_eq!(metadata[Value::from("extra")]["enabled"], Value::Bool(true));
        assert_eq!(body, skill.body);
        assert_eq!(
            get_skill(&root, "old").unwrap().description.as_deref(),
            Some("after")
        );
    }

    #[test]
    fn no_frontmatter_and_description_removal() {
        let (_temp, root) = fixture("Entire body\n");
        let skill = get_skill(&root, "old").unwrap();
        assert_eq!(skill.body, "Entire body\n");
        assert_eq!(skill.description, None);
        save_skill(&root, "old", "old", "description", &skill.body).unwrap();
        save_skill(&root, "old", "old", " \n", &skill.body).unwrap();
        let text = fs::read_to_string(root.join(".unelos/skills/old/SKILL.md")).unwrap();
        assert!(!split(&text)
            .unwrap()
            .0
            .contains_key(Value::from("description")));
        assert_eq!(get_skill(&root, "old").unwrap().body, skill.body);
    }

    #[test]
    fn rename_updates_only_skill_references() {
        let (_temp, root) = fixture("Body");
        fs::create_dir_all(root.join(".unelos/agents/a")).unwrap();
        fs::create_dir_all(root.join(".unelos/workflows")).unwrap();
        let agent =
            "instructions: |\n  skills: [old]\n  old\nskills:\n  - old # old comment\n  - older\n";
        let workflow =
            "tasks:\n  step:\n    instructions: old\n    skills: ['old', other, \"old\"] # old\n";
        fs::write(root.join(".unelos/agents/a/agent.yaml"), agent).unwrap();
        fs::write(root.join(".unelos/workflows/main.yaml"), workflow).unwrap();
        let result = save_skill(&root, "old", "new", "", "Body").unwrap();
        assert_eq!(
            result.updated_references,
            [".unelos/agents/a/agent.yaml", ".unelos/workflows/main.yaml"]
        );
        assert!(!root.join(".unelos/skills/old").exists());
        assert_eq!(get_skill(&root, "new").unwrap().body, "Body");
        assert_eq!(
            fs::read_to_string(root.join(".unelos/agents/a/agent.yaml")).unwrap(),
            agent.replace("- old #", "- \"new\" #")
        );
        assert_eq!(
            fs::read_to_string(root.join(".unelos/workflows/main.yaml")).unwrap(),
            workflow.replace("['old', other, \"old\"]", "[\"new\", other, \"new\"]")
        );
    }

    #[test]
    fn collision_and_invalid_names_leave_source_unchanged() {
        let (_temp, root) = fixture("Body");
        fs::create_dir(root.join(".unelos/skills/taken")).unwrap();
        for name in [
            "taken",
            "",
            ".",
            "..",
            "../escape",
            "a/b",
            "a\\b",
            "a:b",
            "a\0b",
            " old",
            "old ",
        ] {
            assert!(
                save_skill(&root, "old", name, "", "changed").is_err(),
                "{name:?}"
            );
            assert_eq!(
                fs::read_to_string(root.join(".unelos/skills/old/SKILL.md")).unwrap(),
                "Body"
            );
        }
    }

    #[test]
    fn invalid_reference_yaml_is_validated_before_rename() {
        let (_temp, root) = fixture("Body");
        fs::create_dir_all(root.join(".unelos/workflows")).unwrap();
        fs::write(root.join(".unelos/workflows/main.yaml"), "skills: [").unwrap();
        assert!(save_skill(&root, "old", "new", "", "changed").is_err());
        assert_eq!(get_skill(&root, "old").unwrap().body, "Body");
        assert!(!root.join(".unelos/skills/new").exists());
    }
}

pub(crate) fn atomic_write(path: &Utf8Path, text: &str) -> Result<(), String> {
    let temp = path.with_extension("unelos-tmp");
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)
        .map_err(|e| e.to_string())?;
    let result = (|| {
        file.write_all(text.as_bytes())?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temp, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(temp);
    }
    result.map_err(|e: std::io::Error| e.to_string())
}

pub fn save_skill(
    root: &Utf8Path,
    id: &str,
    name: &str,
    description: &str,
    body: &str,
) -> Result<SaveSkillResult, String> {
    valid_id(id)?;
    valid_id(name)?;
    let project = crate::project::absolute_root(root).map_err(|e| e.to_string())?;
    let root = project.join(".unelos");
    let source = bounded(&root, &format!("skills/{id}/SKILL.md"))?;
    let directory = bounded(&root, &format!("skills/{id}"))?;
    let target = bounded(&root, &format!("skills/{name}"))?;
    let text = fs::read_to_string(&source).map_err(|e| e.to_string())?;
    let (mut metadata, _) = split(&text)?;
    metadata.insert(Value::from("name"), Value::from(name));
    metadata.remove(Value::from("description"));
    let description = (!description.trim().is_empty()).then(|| description.to_owned());
    if let Some(description) = &description {
        metadata.insert(Value::from("description"), Value::from(description.clone()));
    }
    let content = format!(
        "---\n{}---\n{body}",
        serde_yaml_ng::to_string(&metadata).map_err(|e| e.to_string())?
    );
    let mut edits = Vec::new();
    if name != id {
        if target.exists() {
            return Err(format!("Skill {name:?} already exists"));
        }
        let mut files = Vec::new();
        for category in ["agents", "tasks", "workflows"] {
            collect(&root, category, &mut files)?;
        }
        files.sort();
        for file in files {
            let original = fs::read_to_string(&file).map_err(|e| e.to_string())?;
            let updated =
                reference_edit(&original, id, name).map_err(|e| format!("{file}: {e}"))?;
            if original != updated {
                edits.push((file, original, updated));
            }
        }
    }
    // Roll back completed writes on ordinary I/O errors; no cross-file crash transaction.
    let mut written = 0;
    let mut renamed = false;
    let result = (|| {
        if name != id {
            fs::rename(&directory, &target).map_err(|e| e.to_string())?;
            renamed = true;
        }
        for (path, _, updated) in &edits {
            atomic_write(path, updated)?;
            written += 1;
        }
        atomic_write(&target.join("SKILL.md"), &content)
    })();
    if let Err(error) = result {
        let mut rollback_errors = Vec::new();
        for (path, original, _) in edits[..written].iter().rev() {
            if let Err(e) = atomic_write(path, original) {
                rollback_errors.push(e);
            }
        }
        if renamed {
            if let Err(e) = fs::rename(&target, &directory) {
                rollback_errors.push(e.to_string());
            }
        }
        return Err(format!("{error}; rollback errors: {rollback_errors:?}"));
    }
    Ok(SaveSkillResult {
        skill: SkillDocument {
            name: name.into(),
            description,
            body: body.into(),
        },
        updated_references: edits
            .iter()
            .map(|(p, _, _)| p.strip_prefix(&project).unwrap().to_string())
            .collect(),
    })
}
