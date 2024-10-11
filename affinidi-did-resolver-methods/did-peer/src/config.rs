use regex::{Captures, Regex};
use serde::{Deserialize, Serialize};
use std::{
    env, fmt,
    fs::File,
    io::{self, BufRead},
    path::{Path, PathBuf},
};

use crate::DIDPeerError;

/// ConfigRaw Struct is used to deserialize the configuration file
/// We then convert this to the CacheConfig Struct
#[derive(Debug, Serialize, Deserialize)]
struct ConfigRaw {
    pub max_did_size_in_kb: String,
    pub max_did_parts: String,
}
#[derive(Clone)]
pub struct Config {
    pub max_did_size_in_kb: f64,
    pub max_did_parts: usize,
}

impl fmt::Debug for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Config")
            .field("max_did_size_in_kb", &self.max_did_size_in_kb)
            .field("max_did_parts", &self.max_did_parts)
            .finish()
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            max_did_size_in_kb: 1.0,
            max_did_parts: 5,
        }
    }
}

impl TryFrom<ConfigRaw> for Config {
    type Error = DIDPeerError;

    fn try_from(raw: ConfigRaw) -> Result<Self, Self::Error> {
        println!("RAW conf: {:?}", raw.max_did_size_in_kb);
        println!("RAW conf: {:?}", raw.max_did_size_in_kb);
        Ok(Config {
            max_did_parts: raw.max_did_parts.parse().unwrap_or(5),
            max_did_size_in_kb: raw.max_did_size_in_kb.parse::<f64>().unwrap_or(1.0),
        })
    }
}

/// Read the primary configuration file for the mediator
/// Returns a ConfigRaw struct, that still needs to be processed for additional information
/// and conversion to Config struct
fn read_config_file(file_name: &str) -> Result<ConfigRaw, DIDPeerError> {
    // Read configuration file parameters
    let raw_config = read_file_lines(file_name)?;

    let config_with_vars = expand_env_vars(&raw_config);
    match toml::from_str(&config_with_vars.join("\n")) {
        Ok(config) => Ok(config),
        Err(err) => Err(DIDPeerError::ConfigError(format!(
            "Could not parse configuration settings. Reason: {:?}",
            err
        ))),
    }
}

/// Reads a file and returns a vector of strings, one for each line in the file.
/// It also strips any lines starting with a # (comments)
/// You can join the Vec back into a single string with `.join("\n")`
pub(crate) fn read_file_lines<P>(file_name: P) -> Result<Vec<String>, DIDPeerError>
where
    P: AsRef<Path>,
{
    let file = File::open(file_name.as_ref()).map_err(|err| {
        DIDPeerError::ConfigError(format!(
            "Could not open file({}). {}",
            file_name.as_ref().display(),
            err
        ))
    })?;

    let mut lines = Vec::new();
    for line in io::BufReader::new(file).lines().map_while(Result::ok) {
        // Strip comments out
        if !line.starts_with('#') {
            lines.push(line);
        }
    }

    Ok(lines)
}

/// Replaces all strings ${VAR_NAME:default_value}
/// with the corresponding environment variables (e.g. value of ${VAR_NAME})
/// or with `default_value` if the variable is not defined.
fn expand_env_vars(raw_config: &Vec<String>) -> Vec<String> {
    let re = Regex::new(r"\$\{(?P<env_var>[A-Z_]{1,}[0-9A-Z_]*):(?P<default_value>.*)\}").unwrap();
    let mut result: Vec<String> = Vec::new();
    for line in raw_config {
        result.push(
            re.replace_all(line, |caps: &Captures| match env::var(&caps["env_var"]) {
                Ok(val) => val,
                Err(_) => (caps["default_value"]).into(),
            })
            .into_owned(),
        );
    }
    result
}

pub fn init() -> Result<Config, DIDPeerError> {
    let cur_working_dir = env::current_dir().unwrap();
    let config_relative_path =
        "affinidi-did-resolver/affinidi-did-resolver-methods/did-peer/conf/did-peer-conf.toml";
    let config_path = _get_relative_path(
        cur_working_dir.as_os_str().to_str().unwrap(),
        &config_relative_path,
    );
    // Read configuration file parameters
    let config_raw = read_config_file(&config_path)?;

    match Config::try_from(config_raw) {
        Ok(parsed_config) => Ok(parsed_config),
        Err(err) => Err(err),
    }
}

fn _get_relative_path(str1: &str, str2: &str) -> String {
    let mut repo_name_flag = false;
    let repo_name = "affinidi-did-resolver";
    // Convert strings into Path objects
    let path1 = Path::new(str1);
    let path2 = Path::new(str2);

    // Iterate through components and find the common part
    let mut up: usize = 0;
    let mut common_part = PathBuf::new();
    let mut path2_iter = path2.components().peekable();
    for component1 in path1.components() {
        if let Some(&component2) = path2_iter.peek() {
            if component1 == component2 {
                common_part.push(component1);
                path2_iter.next();
            } else if common_part.capacity() > 0 && !repo_name_flag {
                if component1.as_os_str().to_str().unwrap() == repo_name {
                    repo_name_flag = true
                }
                up += 1;
            }
        }
    }

    // Remove the common part from the second path
    let mut final_path = PathBuf::new();
    let remaining_part: PathBuf = path2_iter.collect();
    for _ in 0..up {
        final_path.push("..");
    }
    final_path.push(remaining_part);
    final_path.to_str().unwrap_or("").to_string()
}
