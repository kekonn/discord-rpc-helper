
use std::path::PathBuf;
use anyhow::{Result, anyhow, Context};
use scraper::{Selector, ElementRef, Html};
use url::Url;
use tokio::fs::{read_to_string, write};
use html_escape::decode_html_entities;

// XDG RUNTIME HOME

const XDG_RUNTIME_ENV_VAR: &str = "XDG_RUNTIME_DIR";
const CACHE_DIR: &str = "cache";

const STEAM_NAME_SELECTOR: &str = "#appHubAppName";
const STEAM_ICON_SELECTOR: &str = "div.apphub_AppIcon img";

#[derive(Debug, PartialEq, Eq)]
pub struct DocumentCache {
    /// The location of the document cache in the file system
    location: String,
}

impl DocumentCache {

    /// Get a game's name.
    /// 
    /// Returns `anyhow::Result<String>`
    /// 
    /// Parameters:
    /// * `steam_url: &str`: the url of the game's steam store page
    pub async fn get_name(&self, steam_url: &str) -> Result<String> {
        let name_selector = Selector::parse(STEAM_NAME_SELECTOR).unwrap();
        let html = match self.get_steamdb_page(steam_url).await {
            Ok(h) => get_html(&h),
            Err(err) => return Err(anyhow!(err)),
        };
    
        let found_elements: Vec<ElementRef> = html.select(&name_selector).collect();
        match found_elements.len() {
            0 => Err(anyhow!("Could not find any name elements on page")),
            1 => Ok(decode_html_entities(found_elements[0].inner_html().as_str()).to_string()),
            _ => Err(anyhow!("Found more than one name element on the page")),
        }
    }
    
    /// Get a game's app icon
    /// 
    /// Returns `anyhow::Result<String>` as url
    /// 
    /// Parameters:
    /// * `steam_url: &str`: the url of the game's steam store page
    pub async fn get_appicon(&self, steam_url: &str) -> Result<String> {
        let img_selector = Selector::parse(STEAM_ICON_SELECTOR).unwrap();
        let html = self.get_steamdb_page(steam_url).await.map(|h| get_html(&h))?;
    
        let found_elements: Vec<ElementRef> = html.select(&img_selector).collect();
        match found_elements.len() {
            0 => Err(anyhow!("Could not find the icon image on the page")),
            1 => Ok(found_elements[0].value().attr("src").unwrap().to_string()),
            _ => Err(anyhow!("Found more than one app icon on the page")),
        }
    }
    
    /// Downloads the given url, if available
    async fn get_steamdb_page(&self, url: &str) -> Result<String> {
        // Check if cache exists
        let cache_path = self.get_cache_path(url)?;

        match cache_path.try_exists()? {
            true => Ok(read_to_string(&cache_path).await?),
            false => {
                // get supposed cache path
                let document = reqwest::get(url).await?.text().await?;
    
                write(&cache_path, &document).await?;

                Ok(document)
            }
        }
    }

    fn get_cache_path(&self, url: &str) -> Result<PathBuf> {
        let parsed_url = Url::parse(url)?;
        let steamid_url_part = parsed_url.path_segments()
            .with_context(|| format!("Could not find path in url {url}"))?
            .find_map(|p| p.parse::<i64>().ok())
            .with_context(|| format!("Could not find steam id in url {url}"))?;

        let mut path_buff = self.get_location_pathbuf();

        path_buff.push(format!("{steamid_url_part}.html"));

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
            Some(l) => create_cache_dir(l.as_str()).with_context(|| "Error building document cache").map(|p| DocumentCache { location: p}),
            None => create_cache_dir(get_runtime_path().expect("Could not determine XDG_RUNTIME_DIR").as_str()).map(|p| DocumentCache { location: p})
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
    } else {
        path.push(super::constants::APP_NAME);
        path.push(CACHE_DIR);
    }

    match path.is_dir() {
        true => Ok(path.to_string_lossy().to_string()),
        false => std::fs::create_dir_all(&path).with_context(|| format!("Error creating directory '{}'", path.to_string_lossy())).and(Ok(path.to_string_lossy().to_string())),
    }
}

/// Gets the runtime directory
fn get_runtime_path() -> Result<String> {
    std::env::var(XDG_RUNTIME_ENV_VAR).with_context(|| format!("Error reading variable {XDG_RUNTIME_ENV_VAR}"))
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