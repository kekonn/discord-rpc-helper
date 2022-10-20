
use std::{path::{PathBuf, Path}};
use anyhow::{Result, anyhow};
use scraper::{Selector, ElementRef, Html};
use url::Url;

// XDG RUNTIME HOME

const XDG_RUNTIME_ENV_VAR: &str = "XDG_RUNTIME_DIR";
const CACHE_DIR: &str = "cache";

const STEAM_NAME_SELECTOR: &str = "#appHubAppName";
const STEAM_ICON_SELECTOR: &str = "div.apphub_AppIcon img";

#[derive(Debug, PartialEq)]
pub struct DocumentCache {
    /// The location of the document cache in the file system
    location: String,
}

impl DocumentCache {
    async fn get_name(&self, steam_url: &str) -> Result<String> {
        let name_selector = Selector::parse(STEAM_NAME_SELECTOR).unwrap();
        let html = match self.download_steamdb_page(steam_url).await {
            Ok(h) => get_html(&h),
            Err(err) => return Err(anyhow!(err)),
        };
    
        let found_elements: Vec<ElementRef> = html.select(&name_selector).collect();
        match found_elements.len() {
            0 => Err(anyhow!("Could not find any name elements on page")),
            1 => Ok(found_elements[0].inner_html()),
            _ => Err(anyhow!("Found more than one name element on the page")),
        }
    }
    
    async fn get_appicon(&self, steam_url: &str) -> Result<String> {
        let img_selector = Selector::parse(STEAM_ICON_SELECTOR).unwrap();
        let html = match self.download_steamdb_page(steam_url).await {
            Ok(h) => get_html(&h),
            Err(e) => return Err(anyhow!(e)),
        };
    
        let found_elements: Vec<ElementRef> = html.select(&img_selector).collect();
        match found_elements.len() {
            0 => Err(anyhow!("Could not find the icon image on the page")),
            1 => Ok(found_elements[0].value().attr("src").unwrap().to_string()),
            _ => Err(anyhow!("Found more than one app icon on the page")),
        }
    }
    
    /// Downloads the given url or provides it from cache, if available
    async fn download_steamdb_page(&self, url: &str) -> anyhow::Result<String> {
        let document = match reqwest::get(url).await {
            Ok(r) => r.text().await.unwrap(),
            Err(err) => return Err(anyhow!(err)),
        };
    
        Ok(document)
    }

    fn get_cache_path(&self, url: &str) -> Result<PathBuf> {
        let parsed_url = Url::parse(url)?;
        let steamid_url_part = parsed_url.path_segments()
            .unwrap_or_else(|| panic!("Could not find path in url {}", url))
            .find_map(|p| p.parse::<i64>().ok())
            .unwrap_or_else(|| panic!("Could not find steam id in url {}", url));

        let mut path_buff = self.get_location_pathbuf();

        path_buff.push(format!("{}.html", steamid_url_part));

        Ok(path_buff)
    }

    /// Create `PathBuf` from `DocumentCache.location`
    fn get_location_pathbuf(&self) -> PathBuf {
        let mut path_buff = PathBuf::new();

        path_buff.push(&self.location);

        path_buff
    }
}
    
/// Parsing a string into an HTML document
fn get_html(html: &str) -> Html {
    Html::parse_document(html)
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
                match create_cache_dir(l.as_str()) {
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
fn create_cache_dir(path_str: &str) -> Result<String> {
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
    #[ignore = "Running these automatically, they interfere because of the directories"]
    fn builds_with_default_location() -> Result<()> {
        let builder = DocumentCacheBuilder::new();
        let runtime_path = get_runtime_path()?;

        let result = builder.build();

        assert!(result.is_ok(), "Failed to build builder: {}", result.err().unwrap());
        assert!(result?.location == runtime_path);

        Ok(())
    }

    #[test]
    #[ignore = "Running these automatically, they interfere because of the directories"]
    fn builds_with_specced_location() -> Result<()> {
        let builder = DocumentCacheBuilder::new().with_location("./");

        let result = builder.build();

        assert!(result.is_ok(), "Failed to build builder: {}", result.err().unwrap());
        assert_eq!(result?.location, "./discord-rpc-helper/cache");

        Ok(())
    }
}