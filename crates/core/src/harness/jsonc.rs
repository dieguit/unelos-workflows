#[derive(Clone, Copy, PartialEq, Eq)]
enum Version {
    V1,
    V2,
}
use crate::InstallError;
use camino::{Utf8Path, Utf8PathBuf};
use jsonc_parser::{
    cst::{CstInputValue, CstNode, CstObject, CstRootNode},
    ParseOptions,
};
use serde_json::{json, Value};
use std::collections::HashSet;

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
    let names: &[&str] = if root.ends_with(".unelos/opencode") {
        &["unelos", "thin-plugin", "plugin"]
    } else {
        &["unelos"]
    };
    names.iter().any(|name| {
        let directory = root.join(name);
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

pub(crate) fn mcp_text(
    text: &str,
    previous: &Value,
    desired: &Value,
) -> Result<String, InstallError> {
    let document = parse_config(text)?;
    let object = document.object_value().unwrap();
    for section in ["mcp", "tools"] {
        let existing = object
            .get(section)
            .map(|p| {
                p.value().and_then(|n| n.as_object()).ok_or_else(|| {
                    fail(
                        "invalid_opencode_config",
                        format!("{section} must be an object"),
                    )
                })
            })
            .transpose()?;
        let old = previous[section].as_object().unwrap();
        let new = desired[section].as_object().unwrap();
        for (key, value) in old {
            if let Some(property) = existing.as_ref().and_then(|o| o.get(key)) {
                if property.value().and_then(|n| n.to_serde_value()).as_ref() != Some(value) {
                    return Err(fail(
                        "mcp_collision",
                        format!("Managed OpenCode {section}.{key} was edited"),
                    ));
                }
            }
        }
        for key in new.keys() {
            if !old.contains_key(key) && existing.as_ref().is_some_and(|o| o.get(key).is_some()) {
                return Err(fail(
                    "mcp_collision",
                    format!("OpenCode {section}.{key} already exists"),
                ));
            }
        }
        if old.is_empty() && new.is_empty() {
            continue;
        }
        let section_object = if let Some(existing) = existing {
            existing
        } else {
            object.append(section, CstInputValue::Object(vec![]));
            object
                .get(section)
                .unwrap()
                .value()
                .unwrap()
                .as_object()
                .unwrap()
        };
        for key in old.keys().filter(|k| !new.contains_key(*k)) {
            if let Some(property) = section_object.get(key) {
                property.remove();
            }
        }
        for (key, value) in new {
            set(&section_object, key, input(value.clone()));
        }
    }
    Ok(document.to_string())
}

pub fn configured_text(
    text: &str,
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
    let entry = json!({"package":package,"options":options});
    if let Some(existing) = existing {
        if existing.version == Version::V2 && existing.node.to_serde_value() == Some(entry.clone())
        {
            return Ok(text.to_owned());
        }
        if existing.version == Version::V2 && existing.node.as_object().is_some() {
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
        } else if existing.version == Version::V2 {
            let index = existing.node.element_index().unwrap();
            let array = existing.node.parent().unwrap().as_array().unwrap();
            existing.node.remove();
            array.insert(index, input(entry));
        } else {
            existing.node.remove();
            append_entry(&object, input(entry));
        }
    } else {
        append_entry(&object, input(entry));
    }
    Ok(document.to_string())
}

fn append_entry(object: &CstObject, entry: CstInputValue) {
    let key = "plugins";
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

pub fn remove_registration(
    text: &str,
    root: &Utf8Path,
    config: &Utf8Path,
) -> Result<String, InstallError> {
    let document = parse_config(text)?;
    if let Some(entry) = registration(&document, root, config)? {
        entry.node.remove();
        Ok(document.to_string())
    } else {
        Ok(text.to_owned())
    }
}
