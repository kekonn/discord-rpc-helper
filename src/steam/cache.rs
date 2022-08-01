
use std::{path::{PathBuf}};
use anyhow::{Result, anyhow};

// XDG RUNTIME HOME

const XDG_RUNTIME_ENV_VAR: &str = "XDG_RUNTIME_DIR";
const CACHE_DIR: &str = "cache";

#[derive(Debug, PartialEq)]
pub struct DocumentCache {
    location: String,
}

/// Builds a document cache.
/// 
/// Defaults to using `XDG_RUNTIME_DIR`.
pub struct DocumentCacheBuilder {
    location: Option<String>,
}

impl DocumentCacheBuilder {

    /// Creates a new `DocumentCacheBuilder` with default options set.
    /// 
    /// The default location is whatever `XDG_RUNTIME_DIR` points to.
    pub fn new() -> DocumentCacheBuilder {
        DocumentCacheBuilder { location: None }
    }

    /// Changes the location of the document cache
    #[allow(dead_code)]
    pub fn with_location(mut self, path: &str) -> DocumentCacheBuilder {
        self.location = Some(path.to_owned());
        self
    }

    /// Builds the document cache with the given options.
    ///
    /// This consumes the builder.
    pub fn build(self) -> Result<DocumentCache> {
        match self.location {
            Some(l) => {
                match is_valid_location(l.as_str()) {
                    Ok(path) => Ok(DocumentCache { location: path }),
                    Err(e) => Err(anyhow!("Error building document cache: {}", e)),
                }
            },
            None => match get_runtime_path() {
                Ok(p) => Ok(DocumentCache { location: p}),
                Err(e) => Err(e),
            },
        }
    }
}

/// Tries to detect or create a valid cache dir location
fn is_valid_location(path_str: &str) -> Result<String> {
    let mut path = PathBuf::new();
    path.push(path_str);

    if !path.is_dir() { // Also checks if the path exists
        return Err(anyhow!("Path \"{}\" does not point to a directory.", path_str));
    }

    if path.ends_with(super::constants::APP_NAME) {
        path.push(CACHE_DIR);
    } else if !path.ends_with(CACHE_DIR) {
        path.push(super::constants::APP_NAME);
        path.push(CACHE_DIR);
    }

    if path.is_dir() {
        Ok(path.to_string_lossy().to_string())
    } else {
        match std::fs::create_dir_all(&path) {
            Ok(_) => Ok(path.to_string_lossy().to_string()),
            Err(e) => Err(anyhow!(e)),
        }
    }
}

/// Gets the runtime directory
fn get_runtime_path() -> Result<String> {
    match std::env::var(XDG_RUNTIME_ENV_VAR) {
        Ok(path) => Ok(path),
        Err(e) => Err(anyhow!("Error reading variable \"{}\": {}", XDG_RUNTIME_ENV_VAR, e)),
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    // builder tests
    #[test]
    fn can_get_runtime_path_from_env() {
        let result = get_runtime_path();

        assert!(&result.is_ok(), "Found error instead: {}", result.err().unwrap());
    }

    #[test]
    fn is_valid_location_root_fails() {
        let result = is_valid_location("/");

        assert!(result.is_err());
    }

    #[test]
    fn is_valid_location_relative_creates_dirs() {

        let result = is_valid_location("./");

        assert!(result.is_ok(), "{}", result.err().unwrap());
        
        // cleanup
        cleanup_directories().unwrap();
    }

    #[test]
    fn is_valid_location_relative_creates_cache_only() {

        std::fs::create_dir_all("./discord-rpc-helper").unwrap();

        let result = is_valid_location("./discord-rpc-helper");

        assert!(result.is_ok(), "{}", result.err().unwrap());
        
        let created_dir = result.unwrap();
        assert!(created_dir.ends_with("cache"), "Created directory actually is {}", created_dir);

        // cleanup
        cleanup_directories().unwrap();
    }
    
    #[test]
    fn builds_with_default_location() {
        let builder = DocumentCacheBuilder::new();
        let runtime_path = get_runtime_path().unwrap();

        let result = builder.build();

        assert!(result.is_ok(), "Failed to build builder: {}", result.err().unwrap());
        assert!(result.unwrap().location == runtime_path);
    }

    #[test]
    fn builds_with_specced_location() {
        let builder = DocumentCacheBuilder::new().with_location("./");

        let result = builder.build();

        assert!(result.is_ok(), "Failed to build builder: {}", result.err().unwrap());
        assert_eq!(result.unwrap().location, "./discord-rpc-helper/cache");

        cleanup_directories().unwrap();
    }

    /// Cleans locally created directories
    fn cleanup_directories() -> Result<()> {
        let path = Path::new("./discord-rpc-helper");
        
        if path.exists() {
            match std::fs::remove_dir_all(path) {
                Ok(_) => Ok(()),
                Err(e) => Err(anyhow!(e))
            }
        } else {
            Ok(())
        }
    }
}