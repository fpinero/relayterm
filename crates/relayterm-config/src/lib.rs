//! Bounded, versioned configuration parsing into validated domain candidates.

use relayterm_domain::{AgentDefinition, AgentDefinitionId, AgentDefinitionRecord, WorkspaceId};
use serde::Deserialize;
use std::{collections::HashSet, fs::File, io::Read, path::Path, str::FromStr, time::Duration};

pub const MAX_CONFIG_BYTES: usize = 1024 * 1024;
pub const MAX_DEFINITIONS: usize = 128;
pub const TEMPLATE_CATALOG_VERSION: u8 = 1;

const CLAUDE_CODE_TEMPLATE: &str = include_str!("../../../templates/agents/claude-code.toml");
const CODEX_TEMPLATE: &str = include_str!("../../../templates/agents/codex.toml");
const OPENCODE_TEMPLATE: &str = include_str!("../../../templates/agents/opencode.toml");

#[derive(Clone, Eq, PartialEq)]
pub struct AgentTemplate {
    pub key: &'static str,
    pub display_name: String,
    pub command: String,
    pub arguments: Vec<String>,
    pub environment_allowlist: Vec<String>,
    pub capabilities: Vec<String>,
    pub enabled: bool,
    pub document: &'static str,
}

/// Return the small built-in catalog after validating every document through the public parser.
pub fn agent_templates() -> Result<Vec<AgentTemplate>, ConfigError> {
    [
        ("claude_code", CLAUDE_CODE_TEMPLATE),
        ("codex", CODEX_TEMPLATE),
        ("opencode", OPENCODE_TEMPLATE),
    ]
    .into_iter()
    .map(|(key, document)| {
        let parsed = parse(document.as_bytes())?;
        let definition = parsed
            .definitions
            .first()
            .ok_or(ConfigError::InvalidField("definitions"))?;
        if parsed.definitions.len() != 1 {
            return Err(ConfigError::InvalidField("definitions"));
        }
        Ok(AgentTemplate {
            key,
            display_name: definition.display_name.clone(),
            command: definition.command.clone(),
            arguments: definition.arguments.clone(),
            environment_allowlist: definition.environment_allowlist.clone(),
            capabilities: definition.capabilities.clone(),
            enabled: definition.enabled,
            document,
        })
    })
    .collect()
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConfigError {
    TooLarge,
    InvalidSyntax,
    UnsupportedVersion,
    InvalidField(&'static str),
    SensitiveContent,
    DuplicateDefinition,
    Unavailable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WarningCode {
    UnknownField,
    WarningsTruncated,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConfigLocation {
    Root,
    Preferences,
    Storage,
    Definition,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ConfigWarning {
    pub code: WarningCode,
    pub location: ConfigLocation,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ColorMode {
    #[default]
    Auto,
    Always,
    Never,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StorageSettings {
    pub busy_timeout: Duration,
    pub max_connections: u8,
}

impl Default for StorageSettings {
    fn default() -> Self {
        Self {
            busy_timeout: Duration::from_millis(5_000),
            max_connections: 4,
        }
    }
}

pub struct ParsedConfig {
    definitions: Vec<DefinitionDto>,
    pub color_mode: ColorMode,
    pub storage: StorageSettings,
    pub warnings: Vec<ConfigWarning>,
}

impl ParsedConfig {
    pub fn definitions_for(
        &self,
        workspace_id: WorkspaceId,
    ) -> Result<Vec<AgentDefinition>, ConfigError> {
        self.definitions
            .iter()
            .map(|source| {
                let id = AgentDefinitionId::from_str(&source.id)
                    .map_err(|_| ConfigError::InvalidField("definition_id"))?;
                AgentDefinition::restore(AgentDefinitionRecord {
                    id,
                    workspace_id,
                    display_name: source.display_name.clone(),
                    command: source.command.clone(),
                    arguments: source.arguments.clone(),
                    environment_allowlist: source.environment_allowlist.clone(),
                    capabilities: source.capabilities.clone(),
                    enabled: source.enabled,
                })
                .map_err(|error| match error {
                    relayterm_domain::Error::Validation(field) => ConfigError::InvalidField(field),
                    _ => ConfigError::InvalidField("definition"),
                })
            })
            .collect()
    }
}

#[derive(Deserialize)]
struct RootDto {
    format_version: u32,
    #[serde(default)]
    definitions: Vec<DefinitionDto>,
    #[serde(default)]
    preferences: PreferencesDto,
    #[serde(default)]
    storage: StorageDto,
}

#[derive(Clone, Deserialize)]
struct DefinitionDto {
    id: String,
    display_name: String,
    command: String,
    #[serde(default)]
    arguments: Vec<String>,
    #[serde(default)]
    environment_allowlist: Vec<String>,
    #[serde(default)]
    capabilities: Vec<String>,
    #[serde(default = "enabled_default")]
    enabled: bool,
}

fn enabled_default() -> bool {
    true
}

#[derive(Default, Deserialize)]
struct PreferencesDto {
    color_mode: Option<String>,
}

#[derive(Default, Deserialize)]
struct StorageDto {
    busy_timeout_ms: Option<u64>,
    max_connections: Option<u8>,
}

pub fn parse(bytes: &[u8]) -> Result<ParsedConfig, ConfigError> {
    if bytes.len() > MAX_CONFIG_BYTES {
        return Err(ConfigError::TooLarge);
    }
    let source = std::str::from_utf8(bytes).map_err(|_| ConfigError::InvalidSyntax)?;
    reject_sensitive(source)?;
    let value: toml::Value = toml::from_str(source).map_err(|_| ConfigError::InvalidSyntax)?;
    let warnings = inspect_unknown_fields(&value)?;
    let root: RootDto = value.try_into().map_err(|_| ConfigError::InvalidSyntax)?;
    if root.format_version != 1 {
        return Err(ConfigError::UnsupportedVersion);
    }
    if root.definitions.len() > MAX_DEFINITIONS {
        return Err(ConfigError::InvalidField("definitions"));
    }
    let mut ids = HashSet::new();
    for definition in &root.definitions {
        let id = AgentDefinitionId::from_str(&definition.id)
            .map_err(|_| ConfigError::InvalidField("definition_id"))?;
        if !ids.insert(id) {
            return Err(ConfigError::DuplicateDefinition);
        }
    }
    let color_mode = match root.preferences.color_mode.as_deref().unwrap_or("auto") {
        "auto" => ColorMode::Auto,
        "always" => ColorMode::Always,
        "never" => ColorMode::Never,
        _ => return Err(ConfigError::InvalidField("color_mode")),
    };
    let busy_timeout_ms = root.storage.busy_timeout_ms.unwrap_or(5_000);
    if !(1..=30_000).contains(&busy_timeout_ms) {
        return Err(ConfigError::InvalidField("busy_timeout_ms"));
    }
    let max_connections = root.storage.max_connections.unwrap_or(4);
    if !(1..=16).contains(&max_connections) {
        return Err(ConfigError::InvalidField("max_connections"));
    }
    let parsed = ParsedConfig {
        definitions: root.definitions,
        color_mode,
        storage: StorageSettings {
            busy_timeout: Duration::from_millis(busy_timeout_ms),
            max_connections,
        },
        warnings,
    };
    let validation_workspace: WorkspaceId = "00000000-0000-4000-8000-000000000001"
        .parse()
        .map_err(|_| ConfigError::Unavailable)?;
    parsed.definitions_for(validation_workspace)?;
    Ok(parsed)
}

pub fn load(path: &Path) -> Result<ParsedConfig, ConfigError> {
    let file = File::open(path).map_err(|_| ConfigError::Unavailable)?;
    let mut bytes = Vec::new();
    file.take(u64::try_from(MAX_CONFIG_BYTES + 1).map_err(|_| ConfigError::Unavailable)?)
        .read_to_end(&mut bytes)
        .map_err(|_| ConfigError::Unavailable)?;
    parse(&bytes)
}

fn reject_sensitive(source: &str) -> Result<(), ConfigError> {
    let lowered = source.to_ascii_lowercase();
    let patterns = [
        "ghp_",
        "github_pat_",
        "sk-proj-",
        "-----begin private key-----",
        "-----begin rsa private key-----",
        "akia",
    ];
    if patterns.iter().any(|pattern| lowered.contains(pattern)) {
        Err(ConfigError::SensitiveContent)
    } else {
        Ok(())
    }
}

fn inspect_unknown_fields(value: &toml::Value) -> Result<Vec<ConfigWarning>, ConfigError> {
    const MAX_WARNINGS: usize = 32;
    let root = value.as_table().ok_or(ConfigError::InvalidSyntax)?;
    let mut warnings = Vec::new();
    inspect_table(
        root,
        &["format_version", "definitions", "preferences", "storage"],
        ConfigLocation::Root,
        &mut warnings,
    )?;
    if let Some(table) = root.get("preferences").and_then(toml::Value::as_table) {
        inspect_table(
            table,
            &["color_mode"],
            ConfigLocation::Preferences,
            &mut warnings,
        )?;
    }
    if let Some(table) = root.get("storage").and_then(toml::Value::as_table) {
        inspect_table(
            table,
            &["busy_timeout_ms", "max_connections"],
            ConfigLocation::Storage,
            &mut warnings,
        )?;
    }
    if let Some(definitions) = root.get("definitions").and_then(toml::Value::as_array) {
        for definition in definitions {
            let table = definition.as_table().ok_or(ConfigError::InvalidSyntax)?;
            inspect_table(
                table,
                &[
                    "id",
                    "display_name",
                    "command",
                    "arguments",
                    "environment_allowlist",
                    "capabilities",
                    "enabled",
                ],
                ConfigLocation::Definition,
                &mut warnings,
            )?;
        }
    }
    if warnings.len() > MAX_WARNINGS {
        warnings.truncate(MAX_WARNINGS);
        warnings.push(ConfigWarning {
            code: WarningCode::WarningsTruncated,
            location: ConfigLocation::Root,
        });
    }
    Ok(warnings)
}

fn inspect_table(
    table: &toml::Table,
    allowed: &[&str],
    location: ConfigLocation,
    warnings: &mut Vec<ConfigWarning>,
) -> Result<(), ConfigError> {
    for key in table.keys().filter(|key| !allowed.contains(&key.as_str())) {
        let lowered = key.to_ascii_lowercase();
        if [
            "secret",
            "token",
            "password",
            "credential",
            "environment",
            "sql",
            "pragma",
            "extension",
            "permission",
            "acl",
        ]
        .iter()
        .any(|reserved| lowered.contains(reserved))
        {
            return Err(ConfigError::SensitiveContent);
        }
        warnings.push(ConfigWarning {
            code: WarningCode::UnknownField,
            location,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const ID: &str = "00000000-0000-4000-8000-000000000001";

    #[test]
    fn built_in_templates_are_minimal_valid_and_stable() {
        let templates = agent_templates().unwrap();
        assert_eq!(templates.len(), 3);
        assert_eq!(
            templates.iter().map(|item| item.key).collect::<Vec<_>>(),
            ["claude_code", "codex", "opencode"]
        );
        assert_eq!(
            templates
                .iter()
                .map(|item| item.command.as_str())
                .collect::<Vec<_>>(),
            ["claude", "codex", "opencode"]
        );
        for template in templates {
            assert!(!template.enabled);
            assert!(template.arguments.is_empty());
            assert!(template.environment_allowlist.is_empty());
            assert_eq!(template.capabilities, ["terminal"]);
            assert_eq!(
                parse(template.document.as_bytes())
                    .unwrap()
                    .definitions
                    .len(),
                1
            );
            assert!(!template.document.to_ascii_lowercase().contains("token"));
            assert!(!template.document.to_ascii_lowercase().contains("password"));
        }
    }

    #[test]
    fn parses_valid_candidate_and_validates_domain_fields() {
        let source = format!(
            r#"
format_version = 1
unknown_display_hint = true
[preferences]
color_mode = "never"
[storage]
busy_timeout_ms = 1200
max_connections = 2
[[definitions]]
id = "{ID}"
display_name = "Synthetic agent"
command = "agent"
arguments = ["--safe"]
environment_allowlist = ["PATH"]
capabilities = ["terminal"]
enabled = false
"#
        );
        let parsed = parse(source.as_bytes()).unwrap();
        assert_eq!(parsed.color_mode, ColorMode::Never);
        assert_eq!(parsed.storage.max_connections, 2);
        assert_eq!(parsed.warnings.len(), 1);
        let workspace: WorkspaceId = ID.parse().unwrap();
        let definitions = parsed.definitions_for(workspace).unwrap();
        assert!(!definitions[0].record().enabled);
    }

    #[test]
    fn rejects_duplicates_bounds_secrets_and_security_shaped_unknowns() {
        let duplicate = format!(
            r#"format_version = 1
[[definitions]]
id = "{ID}"
display_name = "One"
command = "one"
[[definitions]]
id = "{ID}"
display_name = "Two"
command = "two"
"#
        );
        assert_eq!(
            parse(duplicate.as_bytes()).err(),
            Some(ConfigError::DuplicateDefinition)
        );
        assert_eq!(
            parse(&vec![b'a'; MAX_CONFIG_BYTES + 1]).err(),
            Some(ConfigError::TooLarge)
        );
        assert_eq!(
            parse(b"format_version = 1\napi_token = 'value'").err(),
            Some(ConfigError::SensitiveContent)
        );
        assert_eq!(
            parse(b"format_version = 1\nnote = 'ghp_synthetic'").err(),
            Some(ConfigError::SensitiveContent)
        );
    }

    #[test]
    fn parser_errors_do_not_echo_input() {
        let marker = "PRIVATE_MARKER_42";
        let error = match parse(format!("format_version = [{marker}").as_bytes()) {
            Ok(_) => panic!("invalid input accepted"),
            Err(error) => error,
        };
        assert!(!format!("{error:?}").contains(marker));
    }
}
