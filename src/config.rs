use serde::{Serialize, Deserialize};
use anyhow::{Result, anyhow, Context};
use std::{fs, path::{Path, PathBuf}};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Configuration {
    pub discord_client_id: String
}


impl Configuration {
    /// Create a configuration from a given file.
    /// * `p`: the path of the file.
    ///     
    /// If the given path does not exist in the local directory, we search `$XDG_CONFIG_HOME/discord-rpc-helper/config.json`
    pub fn from_file(p: &str) -> Result<Configuration> {
        let path = match Path::new(p).canonicalize() {
            Ok(r) => r,
            Err(_) => match get_config_path() {
                Ok(r) => r,
                Err(e) => return Err(anyhow!(e))
            }
        };

        let config_path = path.to_str().unwrap();

        let conf_str = fs::read_to_string(config_path)
            .expect(format!("Error reading config file {}", config_path).as_str());

        self::from_string(&conf_str)
    }

    /// Validates the loaded configuration on a functional level.
    /// 
    /// Returns a list of errors (`Vec<&String>`). If the list is empty, then the configuration is valid.
    pub fn validate(&self) -> Vec<String> {
        let mut errors = Vec::<String>::new();

        if self.discord_client_id.is_empty()
        {
            errors.push("discord_client_id is empty.".to_owned());
        }

        return errors;
    }
}

fn get_config_path() -> Result<PathBuf> {
    let config_home = std::env::var("XDG_CONFIG_HOME").with_context(|| format!("Error trying to read env var XDG_CONFIG_HOME"))?;

    let config_path = Path::new(config_home.as_str()).join("discord-rpc-helper").join("config.json");

    match config_path.is_file() {
        true => Ok(config_path),
        false => Err(anyhow!("Could not find path {}/discord-rpc-helper/config.json", config_home))
    }
}

fn from_string(conf_str: &str) -> Result<Configuration> {
    match serde_json::from_str::<Configuration>(conf_str) {
        Ok(c) => Ok(c),
        Err(e) => Err(anyhow!(e)),
    }
}

#[cfg(test)]
mod tests {

    use anyhow::anyhow;

    use super::{get_config_path, Configuration};

    #[test]
    fn detects_invalid_config() {
        let config = Configuration {
            discord_client_id: "".to_string()
        };

        let validation_result = config.validate();

        assert!(!validation_result.is_empty());
    }
    
    #[test]
    fn can_find_config_file() {
        let config_path = get_config_path();

        assert!(config_path.is_ok(), "Error searching for config file: {:?}", config_path.err().unwrap_or(anyhow!("No error")));
    }

    #[test]
    fn can_read_empty_config() {
        let config_str = r#"
            {
                "discord_client_id": ""
            }
        "#;

        let config_res = super::from_string(config_str);

        assert!(config_res.is_ok());

        let config = config_res.unwrap();

        assert!(config.discord_client_id.is_empty());
    }

    #[test]
    fn cannot_read_empty_json() {
        let config_str = r#"
            {
            }
        "#;

        let config_res = super::from_string(config_str);

        assert!(config_res.is_err());
    }

    #[test]
    fn can_read_client_id() {
        let client_id = "5456";

        let config_str = r#"
            {
                "discord_client_id": "5456"
            }
        "#;

        let config = super::from_string(config_str).unwrap();

        assert!(config.discord_client_id == client_id);
    }
}