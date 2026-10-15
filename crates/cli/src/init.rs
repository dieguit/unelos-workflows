use crate::opencode::Version;
use camino::{Utf8Path, Utf8PathBuf};
use jsonc_parser::{
    cst::{CstInputValue, CstNode, CstObject, CstRootNode},
    ParseOptions,
};
use serde_json::{json, Value};
use std::{collections::HashSet, fs, io};
use unelos_core::InstallError;

fn fail(code: &'static str, message: impl Into<String>) -> InstallError {
    InstallError {
        code,
        message: message.into(),
    }
}

fn parse_config(text: &str) -> Result<CstRootNode, InstallError> {
    let options = ParseOptions {
        allow_comments: true,
        allow_trailing_commas: true,
        allow_loose_object_property_names: false,
        allow_missing_commas: false,
        allow_single_quoted_strings: false,
        allow_hexadecimal_numbers: false,
        allow_unary_plus_numbers: false,
        allow_bare_decimal_point_numbers: false,
        allow_non_finite_numbers: false,
        allow_extended_string_escapes: false,
    };
    let document = CstRootNode::parse(text, &options)
        .map_err(|e| fail("invalid_opencode_config", e.to_string()))?;
    let object = document.object_value().ok_or_else(|| {
        fail(
            "invalid_opencode_config",
            "OpenCode config must be an object",
        )
    })?;
    check_keys(&object)?;
    Ok(document)
}

fn check_keys(object: &CstObject) -> Result<(), InstallError> {
    let mut names = HashSet::new();
    for property in object.properties() {
        let name = property
            .decoded_name()
            .ok_or_else(|| fail("invalid_opencode_config", "Invalid property name"))?;
        if !matches!(
            property.name(),
            Some(jsonc_parser::cst::ObjectPropName::String(_))
        ) || !names.insert(name.clone())
        {
            return Err(fail(
                "invalid_opencode_config",
                format!("Unquoted or duplicate property: {name}"),
            ));
        }
        if let Some(node) = property.value() {
            check_node_keys(&node)?;
        }
    }
    Ok(())
}

fn check_node_keys(node: &CstNode) -> Result<(), InstallError> {
    if let Some(object) = node.as_object() {
        check_keys(&object)?;
    }
    if let Some(array) = node.as_array() {
        for child in array.elements() {
            check_node_keys(&child)?;
        }
    }
    Ok(())
}

fn managed_path(package: &str, root: &Utf8Path, config: &Utf8Path) -> bool {
    let path = if package.starts_with("file:") {
        let Ok(url) = url::Url::parse(package) else {
            return false;
        };
        let Ok(path) = url.to_file_path() else {
            return false;
        };
        let Ok(path) = Utf8PathBuf::from_path_buf(path) else {
            return false;
        };
        path
    } else {
        let path = Utf8Path::new(package);
        if !path.is_absolute() && !package.starts_with('.') {
            return false;
        }
        config.parent().unwrap_or(root).join(path)
    };
    let mut normalized = Utf8PathBuf::new();
    for part in path.components() {
        match part {
            camino::Utf8Component::CurDir => {}
            camino::Utf8Component::ParentDir => {
                normalized.pop();
            }
            _ => normalized.push(part.as_str()),
        }
    }
    ["unelos", "thin-plugin", "plugin"].iter().any(|name| {
        let directory = root.join(".unelos/opencode").join(name);
        normalized == directory || normalized == directory.join("index.js")
    })
}

struct Registration {
    version: Version,
    node: CstNode,
    options: Value,
}

fn registration(
    document: &CstRootNode,
    root: &Utf8Path,
    config: &Utf8Path,
) -> Result<Option<Registration>, InstallError> {
    let object = document.object_value().unwrap();
    let mut found = None;
    for (key, version) in [("plugins", Version::V2), ("plugin", Version::V1)] {
        let Some(property) = object.get(key) else {
            continue;
        };
        let array = property
            .value()
            .and_then(|v| v.as_array())
            .ok_or_else(|| fail("invalid_opencode_config", format!("{key} must be an array")))?;
        for node in array.elements() {
            let value = node
                .to_serde_value()
                .ok_or_else(|| fail("invalid_opencode_config", "Invalid plugin entry"))?;
            let (package, options) = match &value {
                Value::String(package) => (Some(package.as_str()), None),
                Value::Object(entry) => (
                    entry.get("package").and_then(Value::as_str),
                    entry.get("options"),
                ),
                Value::Array(entry) => (entry.first().and_then(Value::as_str), entry.get(1)),
                _ => (None, None),
            };
            if !package.is_some_and(|p| managed_path(p, root, config)) {
                continue;
            }
            if found.is_some() {
                return Err(fail(
                    "ambiguous_integration",
                    "Multiple Unelos plugin registrations",
                ));
            }
            let options = options.cloned().unwrap_or_else(|| json!({}));
            if !options.is_object() {
                return Err(fail(
                    "invalid_opencode_config",
                    "Unelos options must be an object",
                ));
            }
            if matches!(&value, Value::Array(items) if items.len() > 2) {
                return Err(fail(
                    "invalid_opencode_config",
                    "Invalid Unelos plugin tuple",
                ));
            }
            found = Some(Registration {
                version,
                node,
                options,
            });
        }
    }
    Ok(found)
}

fn input(value: Value) -> CstInputValue {
    match value {
        Value::Null => CstInputValue::Null,
        Value::Bool(v) => v.into(),
        Value::Number(v) => CstInputValue::Number(v.to_string()),
        Value::String(v) => v.into(),
        Value::Array(v) => CstInputValue::Array(v.into_iter().map(input).collect()),
        Value::Object(v) => {
            CstInputValue::Object(v.into_iter().map(|(k, v)| (k, input(v))).collect())
        }
    }
}

fn set(object: &CstObject, key: &str, value: CstInputValue) {
    if let Some(property) = object.get(key) {
        property.set_value(value);
    } else {
        object.append(key, value);
    }
}

fn configured_text(
    text: &str,
    version: Version,
    package: &str,
    binary: &str,
    root: &Utf8Path,
    config: &Utf8Path,
) -> Result<String, InstallError> {
    let document = parse_config(text)?;
    let object = document.object_value().unwrap();
    let existing = registration(&document, root, config)?;
    let mut options = existing
        .as_ref()
        .map(|r| r.options.clone())
        .unwrap_or_else(|| json!({}));
    options["binary"] = binary.into();
    let package = if version == Version::V1 {
        format!("{package}/index.js")
    } else {
        package.into()
    };
    let entry = if version == Version::V2 {
        json!({"package":package,"options":options})
    } else {
        json!([package, options])
    };
    if let Some(existing) = existing {
        if existing.version == version && existing.node.to_serde_value() == Some(entry.clone()) {
            return Ok(text.to_owned());
        }
        if existing.version == version
            && version == Version::V2
            && existing.node.as_object().is_some()
        {
            let node = existing.node.as_object().unwrap();
            set(&node, "package", package.into());
            let options_node = node
                .get("options")
                .and_then(|p| p.value())
                .and_then(|n| n.as_object());
            if let Some(options_node) = options_node {
                set(&options_node, "binary", binary.into());
            } else {
                set(&node, "options", input(options));
            }
        } else if existing.version == version {
            let index = existing.node.element_index().unwrap();
            let array = existing.node.parent().unwrap().as_array().unwrap();
            existing.node.remove();
            array.insert(index, input(entry));
        } else {
            existing.node.remove();
            append_entry(&object, version, input(entry));
        }
    } else {
        append_entry(&object, version, input(entry));
    }
    if object.get("$schema").is_none() {
        object.append("$schema", "https://opencode.ai/config.json".into());
    }
    Ok(document.to_string())
}

fn append_entry(object: &CstObject, version: Version, entry: CstInputValue) {
    let key = if version == Version::V2 {
        "plugins"
    } else {
        "plugin"
    };
    if let Some(array) = object
        .get(key)
        .and_then(|p| p.value())
        .and_then(|n| n.as_array())
    {
        array.append(entry);
    } else {
        object.append(key, CstInputValue::Array(vec![entry]));
    }
}

const METADATA: &str = ".unelos/integration.json";
const PLUGIN: &str = ".unelos/opencode/unelos";
const COMMAND: &str = ".opencode/commands/unelos.md";
const COMMAND_BYTES: &[u8] = include_bytes!("../../../integrations/opencode/commands/unelos.md");
const PLUGIN_FILES: &[(&str, &[u8])] = &[
    (
        "package.json",
        include_bytes!("../../../integrations/opencode/package.json"),
    ),
    (
        "index.js",
        include_bytes!("../../../integrations/opencode/index.js"),
    ),
    (
        "v1.js",
        include_bytes!("../../../integrations/opencode/v1.js"),
    ),
    ("commands/unelos.md", COMMAND_BYTES),
];

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Integration {
    opencode: Version,
}

fn io_error(e: impl std::fmt::Display) -> InstallError {
    fail("install_io_failed", e.to_string())
}

fn absolute(root: &Utf8Path) -> Result<Utf8PathBuf, InstallError> {
    let path = fs::canonicalize(root).map_err(io_error)?;
    let path = Utf8PathBuf::from_path_buf(path)
        .map_err(|_| fail("invalid_path", "Project path must be UTF-8"))?;
    if !path.is_dir() {
        return Err(fail(
            "invalid_project",
            "Project must be an existing directory",
        ));
    }
    Ok(path)
}

fn read_optional(root: &Utf8Path, relative: &str) -> Result<Option<Vec<u8>>, InstallError> {
    unelos_core::check_install_path(root, Utf8Path::new(relative))?;
    match fs::read(root.join(relative)) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(io_error(e)),
    }
}

fn metadata(root: &Utf8Path) -> Result<Option<Integration>, InstallError> {
    read_optional(root, METADATA)?
        .map(|bytes| {
            serde_json::from_slice(&bytes)
                .map_err(|e| fail("invalid_integration", format!("{METADATA}: {e}")))
        })
        .transpose()
}

fn config_path(root: &Utf8Path) -> Result<Utf8PathBuf, InstallError> {
    let mut found = Vec::new();
    for relative in [
        "opencode.json",
        "opencode.jsonc",
        ".opencode/opencode.json",
        ".opencode/opencode.jsonc",
    ] {
        unelos_core::check_install_path(root, Utf8Path::new(relative))?;
        match fs::symlink_metadata(root.join(relative)) {
            Ok(_) => found.push(root.join(relative)),
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => return Err(io_error(e)),
        }
    }
    if found.len() > 1 {
        return Err(fail(
            "ambiguous_opencode_config",
            format!(
                "Multiple project configs: {}",
                found
                    .iter()
                    .map(|p| p.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        ));
    }
    Ok(found.pop().unwrap_or_else(|| root.join("opencode.json")))
}

fn config_text(path: &Utf8Path) -> Result<String, InstallError> {
    match fs::read_to_string(path) {
        Ok(text) => Ok(text),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok("{}\n".into()),
        Err(e) => Err(io_error(e)),
    }
}

fn check_file(root: &Utf8Path, relative: &Utf8Path) -> Result<(), InstallError> {
    unelos_core::check_install_path(root, relative)?;
    match fs::metadata(root.join(relative)) {
        Ok(m) if !m.is_file() => Err(fail("invalid_path", format!("Expected file: {relative}"))),
        Ok(_) => Ok(()),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(io_error(e)),
    }
}

pub fn initialize(root: &Utf8Path, requested: Option<Version>) -> Result<Value, InstallError> {
    let root = absolute(root)?;
    unelos_core::check_project_init(&root)?;
    let recorded = metadata(&root)?;
    let version = requested
        .or(recorded.as_ref().map(|m| m.opencode))
        .unwrap_or(Version::V2);
    let config = config_path(&root)?;
    let text = config_text(&config)?;
    let document = parse_config(&text)?;
    let existing = registration(&document, &root, &config)?;
    if requested.is_none()
        && recorded
            .as_ref()
            .zip(existing.as_ref())
            .is_some_and(|(m, r)| m.opencode != r.version)
    {
        return Err(fail("integration_conflict", "Recorded OpenCode version differs from plugin registration; select --opencode explicitly"));
    }
    let executable =
        fs::canonicalize(std::env::current_exe().map_err(io_error)?).map_err(io_error)?;
    let executable = Utf8PathBuf::from_path_buf(executable)
        .map_err(|_| fail("invalid_path", "CLI executable path must be UTF-8"))?;
    let package = if config.parent() == Some(root.as_path()) {
        "./.unelos/opencode/unelos"
    } else {
        "../.unelos/opencode/unelos"
    };
    let edited = configured_text(&text, version, package, executable.as_str(), &root, &config)?;
    for (file, _) in PLUGIN_FILES {
        check_file(&root, &Utf8PathBuf::from(PLUGIN).join(file))?;
    }
    for path in [METADATA, COMMAND] {
        check_file(&root, Utf8Path::new(path))?;
    }
    check_file(&root, config.strip_prefix(&root).unwrap())?;
    let command = read_optional(&root, COMMAND)?;
    if version == Version::V1 && command.as_deref().is_some_and(|b| b != COMMAND_BYTES) {
        return Err(fail(
            "import_collision",
            "Existing .opencode/commands/unelos.md differs; preserve or move it before V1 init",
        ));
    }
    let recorded_bytes =
        serde_json::to_vec_pretty(&Integration { opencode: version }).map_err(io_error)?;
    unelos_core::initialize_project(&root)?;
    for (file, bytes) in PLUGIN_FILES {
        unelos_core::write_install_file(&root, &Utf8PathBuf::from(PLUGIN).join(file), bytes)?;
    }
    if version == Version::V1 {
        unelos_core::write_install_file(&root, Utf8Path::new(COMMAND), COMMAND_BYTES)?;
    }
    unelos_core::write_install_file(
        &root,
        config.strip_prefix(&root).unwrap(),
        edited.as_bytes(),
    )?;
    if version == Version::V2 && command.as_deref() == Some(COMMAND_BYTES) {
        fs::remove_file(root.join(COMMAND)).map_err(io_error)?;
    }
    unelos_core::write_install_file(&root, Utf8Path::new(METADATA), &recorded_bytes)?;
    Ok(
        json!({"opencode":version,"project":root,"config":config,"plugin":root.join(PLUGIN),
        "binary":executable,"metadata":root.join(METADATA),"message":"Open a fresh OpenCode session, then install your workflow."}),
    )
}

pub fn install_version(
    root: &Utf8Path,
    requested: Option<Version>,
) -> Result<Option<Version>, InstallError> {
    let recorded = metadata(root)?;
    let Some(recorded) = recorded else {
        return Ok(requested);
    };
    let root = absolute(root)?;
    let repair = || {
        fail(
            "integration_missing",
            "Unelos integration is incomplete. Run unelos init to repair the integration",
        )
    };
    let config = config_path(&root)?;
    if !config.is_file() {
        return Err(repair());
    }
    let document = parse_config(&config_text(&config)?)?;
    let registered = registration(&document, &root, &config)?.ok_or_else(repair)?;
    if registered.version != recorded.opencode {
        return Err(fail("integration_conflict", "Recorded OpenCode version differs from plugin registration. Run unelos init --opencode v1 or v2 to resolve it"));
    }
    for (file, _) in PLUGIN_FILES {
        let relative = Utf8PathBuf::from(PLUGIN).join(file);
        check_file(&root, &relative)?;
        if !root.join(relative).is_file() {
            return Err(repair());
        }
    }
    if recorded.opencode == Version::V1 && read_optional(&root, COMMAND)?.is_none() {
        return Err(repair());
    }
    Ok(Some(requested.unwrap_or(recorded.opencode)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn edit(text: &str, version: Version) -> Result<String, InstallError> {
        configured_text(
            text,
            version,
            "./.unelos/opencode/unelos",
            "/bin/unelos",
            Utf8Path::new("/project"),
            Utf8Path::new("/project/opencode.jsonc"),
        )
    }

    #[test]
    fn config_preserves_unrelated_content() {
        let text =
            "{\n // keep comment\n \"model\": \"p/m\",\n \"plugins\": [\"other-plugin\",],\n}\n";
        let edited = edit(text, Version::V2).unwrap();
        assert!(edited.contains("// keep comment"));
        assert!(edited.contains("other-plugin"));
        assert!(edited.contains("./.unelos/opencode/unelos"));
        assert_eq!(edit(&edited, Version::V2).unwrap(), edited);
    }

    #[test]
    fn malformed_or_duplicate_configuration_is_rejected() {
        for text in ["[]", "null", "", "{plugins: []}",
            "{\"plugins\":[],\"plugins\":[]}", "{\"plugins\":false}",
            "{\"plugins\":[{\"package\":\"./.unelos/opencode/unelos\",\"options\":false}]}",
            "{\"plugins\":[\"./.unelos/opencode/unelos\",\"./.unelos/opencode/thin-plugin\"]}",
            "{\"plugins\":[\"./.unelos/opencode/unelos\"],\"plugin\":[\"./.unelos/opencode/plugin/index.js\"]}",
        ] {
            assert!(edit(text, Version::V2).is_err(), "accepted {text}");
        }
    }

    #[test]
    fn json5_and_missing_commas_are_not_valid_opencode_config() {
        for text in [
            "{\"model\":'p/m'}",
            "{\"a\":1 \"b\":2}",
            "{\"a\":0xff}",
            "{\"a\":+1}",
            "{\"a\":.5}",
            "{\"a\":NaN}",
            "{\"a\":\"\\x41\"}",
        ] {
            assert!(edit(text, Version::V2).is_err(), "accepted {text}");
        }
    }

    #[test]
    fn local_legacy_registration_migrates_and_preserves_options() {
        for path in [
            "./.unelos/opencode/thin-plugin",
            "/project/.unelos/opencode/plugin",
            "file:///project/.unelos/opencode/thin-plugin",
            "./x/../.unelos/opencode/unelos",
        ] {
            let text = serde_json::json!({"plugins":["unelos", {"package":path,"options":{"custom":true,"binary":"old"}}]}).to_string();
            let edited = edit(&text, Version::V2).unwrap();
            let value: serde_json::Value =
                jsonc_parser::parse_to_serde_value(&edited, &jsonc_parser::ParseOptions::default())
                    .unwrap();
            assert_eq!(value["plugins"].as_array().unwrap().len(), 2);
            assert_eq!(value["plugins"][0], "unelos");
            assert_eq!(value["plugins"][1]["options"]["custom"], true);
            assert_eq!(value["plugins"][1]["options"]["binary"], "/bin/unelos");
            assert_eq!(value["plugins"][1]["package"], "./.unelos/opencode/unelos");
        }
    }

    #[test]
    fn version_switch_removes_only_managed_registration() {
        let text =
            "{\"plugin\":[\"other\",[\"./.unelos/opencode/plugin/index.js\",{\"custom\":1}]]}";
        let edited = edit(text, Version::V2).unwrap();
        let v2: serde_json::Value =
            jsonc_parser::parse_to_serde_value(&edited, &jsonc_parser::ParseOptions::default())
                .unwrap();
        assert_eq!(v2["plugin"], serde_json::json!(["other"]));
        assert_eq!(v2["plugins"][0]["options"]["custom"], 1);
        let switched = edit(&edited, Version::V1).unwrap();
        let v1: serde_json::Value =
            jsonc_parser::parse_to_serde_value(&switched, &jsonc_parser::ParseOptions::default())
                .unwrap();
        assert_eq!(v1["plugin"][1][0], "./.unelos/opencode/unelos/index.js");
        assert_eq!(v1["plugins"], serde_json::json!([]));
    }
}
