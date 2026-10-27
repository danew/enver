use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};
use url::Url;

#[derive(Parser)]
#[command(
    name = "envctl",
    about = "Read and update .env values without exposing private secrets"
)]
struct Cli {
    #[arg(short, long, default_value = ".env", global = true)]
    file: PathBuf,
    #[arg(long, global = true)]
    config: Option<PathBuf>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Read a value, or metadata if it is private.
    Get { key: String },
    /// List keys and metadata. Values are only included for public keys.
    List,
    /// Set a value and its explicit visibility.
    Set {
        key: String,
        value: String,
        #[arg(long, value_enum, default_value_t = Visibility::Private)]
        visibility: Visibility,
    },
    /// Print the effective visibility and metadata without printing private values.
    Inspect { key: String },
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize, ValueEnum)]
#[serde(rename_all = "lowercase")]
enum Visibility {
    Public,
    Private,
}

#[derive(Debug, Default, Deserialize)]
struct Config {
    #[serde(default)]
    public_key_patterns: Vec<String>,
    #[serde(default)]
    public_value_patterns: Vec<String>,
    #[serde(default)]
    private_key_patterns: Vec<String>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
struct Metadata {
    exists: bool,
    #[serde(rename = "type")]
    value_type: String,
    format: String,
    visibility: Visibility,
    #[serde(skip_serializing_if = "Option::is_none")]
    value: Option<String>,
}

impl Default for Visibility {
    fn default() -> Self {
        Self::Private
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let config = load_config(cli.config.as_deref())?;
    let mut env = read_env(&cli.file)?;
    let overrides = read_overrides(&cli.file)?;

    match cli.command {
        Command::Get { key } | Command::Inspect { key } => {
            let result = describe(&key, env.get(&key), &config, overrides.get(&key));
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Command::List => {
            let result: BTreeMap<_, _> = env
                .keys()
                .map(|key| {
                    (
                        key.clone(),
                        describe(key, env.get(key), &config, overrides.get(key)),
                    )
                })
                .collect();
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Command::Set {
            key,
            value,
            visibility,
        } => {
            validate_key(&key)?;
            write_env_value(&cli.file, &key, &value)?;
            env.insert(key.clone(), value);
            let mut overrides = overrides;
            overrides.insert(key, visibility);
            write_overrides(&cli.file, &overrides)?;
        }
    }
    Ok(())
}

fn describe(
    key: &str,
    value: Option<&String>,
    config: &Config,
    override_visibility: Option<&Visibility>,
) -> Metadata {
    let Some(value) = value else {
        return Metadata {
            exists: false,
            ..Default::default()
        };
    };
    let visibility = override_visibility
        .copied()
        .unwrap_or_else(|| effective_visibility(key, value, config));
    Metadata {
        exists: true,
        value_type: value_type(value).to_string(),
        format: value_format(value).to_string(),
        value: (visibility == Visibility::Public).then(|| value.clone()),
        visibility,
    }
}

fn effective_visibility(key: &str, value: &str, config: &Config) -> Visibility {
    if matches_any(key, &config.private_key_patterns) {
        return Visibility::Private;
    }
    if matches_any(key, &config.public_key_patterns)
        || matches_any(value, &config.public_value_patterns)
        || is_uri(value)
    {
        Visibility::Public
    } else {
        Visibility::Private
    }
}

fn matches_any(value: &str, patterns: &[String]) -> bool {
    patterns
        .iter()
        .filter_map(|pattern| Regex::new(pattern).ok())
        .any(|regex| regex.is_match(value))
}

fn is_uri(value: &str) -> bool {
    Url::parse(value)
        .map(|url| url.scheme().len() > 1)
        .unwrap_or(false)
}

fn value_type(value: &str) -> &'static str {
    if value.eq_ignore_ascii_case("true") || value.eq_ignore_ascii_case("false") {
        "boolean"
    } else if value.parse::<i64>().is_ok() {
        "integer"
    } else if value.parse::<f64>().is_ok() {
        "number"
    } else if value.trim_start().starts_with('[') {
        "array"
    } else if is_uri(value) {
        "uri"
    } else {
        "string"
    }
}

fn value_format(value: &str) -> &'static str {
    if value.trim_start().starts_with('[') && value.trim_end().ends_with(']') {
        "json-array"
    } else if value.contains(',') {
        "comma-separated"
    } else if is_uri(value) {
        "uri"
    } else {
        "scalar"
    }
}

fn read_env(path: &Path) -> Result<BTreeMap<String, String>> {
    let text =
        fs::read_to_string(path).with_context(|| format!("cannot read {}", path.display()))?;
    let mut values = BTreeMap::new();
    for (line_number, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        validate_key(key.trim())
            .with_context(|| format!("invalid key on line {}", line_number + 1))?;
        values.insert(key.trim().to_string(), unquote(value.trim()));
    }
    Ok(values)
}

fn write_env_value(path: &Path, key: &str, value: &str) -> Result<()> {
    let original = fs::read_to_string(path).unwrap_or_default();
    let replacement = format!("{}={}", key, quote_if_needed(value));
    let mut found = false;
    let mut lines = Vec::new();
    for line in original.lines() {
        let trimmed = line.trim_start();
        if !trimmed.starts_with('#')
            && trimmed
                .split_once('=')
                .map(|(k, _)| k.trim() == key)
                .unwrap_or(false)
        {
            lines.push(replacement.clone());
            found = true;
        } else {
            lines.push(line.to_string());
        }
    }
    if !found {
        lines.push(replacement);
    }
    fs::write(path, format!("{}\n", lines.join("\n")))
        .with_context(|| format!("cannot write {}", path.display()))?;
    Ok(())
}

fn sidecar(path: &Path) -> PathBuf {
    PathBuf::from(format!("{}.visibility.json", path.display()))
}
fn read_overrides(path: &Path) -> Result<BTreeMap<String, Visibility>> {
    let file = sidecar(path);
    if !file.exists() {
        return Ok(BTreeMap::new());
    }
    Ok(serde_json::from_str(&fs::read_to_string(file)?)?)
}
fn write_overrides(path: &Path, overrides: &BTreeMap<String, Visibility>) -> Result<()> {
    fs::write(
        sidecar(path),
        serde_json::to_string_pretty(overrides)? + "\n",
    )?;
    Ok(())
}
fn load_config(path: Option<&Path>) -> Result<Config> {
    let path = path
        .map(PathBuf::from)
        .or_else(|| dirs::config_dir().map(|p| p.join("envctl/config.toml")));
    let Some(path) = path else {
        return Ok(Config::default());
    };
    if !path.exists() {
        return Ok(Config::default());
    }
    Ok(toml::from_str(&fs::read_to_string(path)?)?)
}
fn validate_key(key: &str) -> Result<()> {
    if key.is_empty() || !key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        bail!("invalid environment key: {key}");
    }
    Ok(())
}
fn unquote(value: &str) -> String {
    value
        .strip_prefix('"')
        .and_then(|v| v.strip_suffix('"'))
        .or_else(|| value.strip_prefix('\'').and_then(|v| v.strip_suffix('\'')))
        .unwrap_or(value)
        .to_string()
}
fn quote_if_needed(value: &str) -> String {
    if value.is_empty()
        || value
            .chars()
            .any(|c| c.is_whitespace() || c == '#' || c == '"')
    {
        format!("\"{}\"", value.replace('"', "\\\""))
    } else {
        value.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn uri_values_are_public() {
        assert_eq!(
            effective_visibility("X", "https://example.com", &Config::default()),
            Visibility::Public
        );
    }
    #[test]
    fn ordinary_values_are_private() {
        assert_eq!(
            effective_visibility("X", "secret", &Config::default()),
            Visibility::Private
        );
    }
    #[test]
    fn private_rule_wins() {
        let config = Config {
            public_key_patterns: vec!["TOKEN".into()],
            private_key_patterns: vec!["^TOKEN$".into()],
            ..Default::default()
        };
        assert_eq!(
            effective_visibility("TOKEN", "x", &config),
            Visibility::Private
        );
    }
}
