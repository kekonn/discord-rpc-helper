
use std::{path::PathBuf, sync::Arc, fs::File, io::{BufReader, BufWriter}, collections::HashMap};
use anyhow::{Result, anyhow, Context};
use scraper::{Selector, ElementRef, Html};
use url::Url;
use tokio::fs::{read_to_string, write };
use html_escape::decode_html_entities;
use reqwest_cookie_store::{CookieStore, CookieStoreMutex};

// XDG RUNTIME HOME

const XDG_RUNTIME_ENV_VAR: &str = "XDG_RUNTIME_DIR";
const CACHE_DIR: &str = "cache";
const COOKIE_STORE_PATH: &str = "cookies.json";

const STEAM_NAME_SELECTOR: &str = "#appHubAppName";
const STEAM_ICON_SELECTOR: &str = "div.apphub_AppIcon img";

const AGEGATE_SELECTOR: &str = "div.agegate_birthday_selector";
const AGESET_BASE_URL: &str = "https://store.steampowered.com/agecheckset/app/";

const SESSION_ID_COOKIE_NAME: &str = "sessionid";
const SESSION_ID_COOKIE_DOMAIN: &str = "store.steampowered.com";

#[derive(Debug)]
pub struct DocumentCache {
    /// The location of the document cache in the file system
    location: String,
    cookies: Arc<CookieStoreMutex>,
}

impl DocumentCache {

    /// Creates a new [DocumentCache](#DocumentCache) with the given location.
    pub fn new(cache_loc: String) -> Self {
        let mut location = PathBuf::new();
        location.push(&cache_loc);
        
        let cookie_store = {
            location.push(COOKIE_STORE_PATH);
            if let Ok(file) = File::open(&location).map(BufReader::new)
            {
                CookieStore::load_json(file).unwrap()
            } else {
                CookieStore::new(None)
            }
        };
        
        let cookie_store = CookieStoreMutex::new(cookie_store);
        let cookie_store = Arc::new(cookie_store);

        Self { location: cache_loc, cookies: cookie_store }
    }

    /// Get a game's name.
    /// 
    /// Returns `anyhow::Result<String>`
    /// 
    /// Parameters:
    /// * `steam_url: &str`: the url of the game's steam store page
    pub async fn get_name(&self, steam_url: &str) -> Result<String> {
        let name_selector = Selector::parse(STEAM_NAME_SELECTOR).unwrap();
        let html = match self.get_steam_page(steam_url).await {
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
        let html = self.get_steam_page(steam_url).await.map(|h| get_html(&h))?;
    
        let found_elements: Vec<ElementRef> = html.select(&img_selector).collect();
        match found_elements.len() {
            0 => Err(anyhow!("Could not find the icon image on the page")),
            1 => Ok(found_elements[0].value().attr("src").unwrap().to_string()),
            _ => Err(anyhow!("Found more than one app icon on the page")),
        }
    }
    
    /// Downloads the given url, if available
    async fn get_steam_page(&self, url: &str) -> Result<String> {
        // Check if cache exists
        let cache_path = self.get_cache_path(url)?;

        match cache_path.try_exists()? {
            true => Ok(read_to_string(&cache_path).await?),
            false => {
                // get supposed cache path
                let document = self.download_steam_page(url).await?;
    
                write(&cache_path, &document).await?;

                Ok(document)
            }
        }
    }

    async fn download_steam_page(&self, url: &str) -> Result<String> {
        let rest_client = self.build_client().with_context(|| "Error building rest client for cache")?;

        let request = rest_client.get(url).build()?;
        let response = rest_client.execute(request).await?;
        

        

        let (resp_content, is_age_gate) ={ 
            let resp_html = get_html(response.text().await?.as_str());

            let is_age_gate = {
                let gate_selector = Selector::parse(AGEGATE_SELECTOR).unwrap();
                let mut age_gate_div = resp_html.select(&gate_selector);
    
                age_gate_div.next().is_some()
            };

            (resp_html.html(), is_age_gate)
        };

        if is_age_gate {
            let app_id = Self::get_appid_from_url(url)?;
            let resp_content = self.handle_agegate(app_id, &rest_client).await?;
            Ok(resp_content)
        }  else {
            Ok(resp_content)
        }
    }

    fn build_client(&self) -> Result<reqwest::Client> {
        reqwest::ClientBuilder::new()
                .cookie_provider(Arc::clone(&self.cookies)).
                build().with_context(|| "Error building reqwest client")
    }

    fn get_session_cookie_value(&self) -> Result<String> {
        let cookies = self.cookies.lock().unwrap();

        if let Some(session_cookie) = cookies.get(SESSION_ID_COOKIE_DOMAIN, "/", SESSION_ID_COOKIE_NAME) {
            let cookie_value = session_cookie.value();
            Ok(cookie_value.to_owned())
        } else {
            Err(anyhow!("We encountered an age gate, but did not manage to capture a session cookie yet"))
        }
    }
    
    async fn handle_agegate(&self, app_id: i64, client: &reqwest::Client) -> Result<String> {
        let session_id = self.get_session_cookie_value()?;
        
        // time to lie about our age (or not, in some freak occurrences)
        let mut ageset_form = HashMap::new();
        ageset_form.insert("sessionid", session_id.as_str());
        ageset_form.insert("ageDay", "1");
        ageset_form.insert("ageMonth", "January");
        ageset_form.insert("ageYear", "1990");

        let ageset_post = client.post(format!("{}{}", AGESET_BASE_URL, app_id))
                .form(&ageset_form)
                .build()?;

        let ageset_resp = client.execute(ageset_post).await?;

        todo!()
    }

    fn get_cache_path(&self, url: &str) -> Result<PathBuf> {
        let steamid_url_part = Self::get_appid_from_url(url)?;
        let mut path_buff = self.get_location_pathbuf();

        path_buff.push(format!("{steamid_url_part}.html"));

        Ok(path_buff)
    }

    fn get_appid_from_url(url: &str) -> Result<i64> {
        let parsed_url = Url::parse(url)?;

        parsed_url.path_segments()
            .with_context(|| format!("Could not find path in url {url}"))?
            .find_map(|p| p.parse::<i64>().ok())
            .with_context(|| format!("Could not find steam id in url {url}"))
    }

    /// Create `PathBuf` from `DocumentCache.location`
    fn get_location_pathbuf(&self) -> PathBuf {
        let mut path_buff = PathBuf::new();

        path_buff.push(&self.location);

        path_buff
    }

    /// Save cookies to temp location.
    /// 
    /// This allows to reuse age gates after app restarts.
    fn save_cookies(&self) {
        let mut writer = {
            let mut cache_path = self.get_location_pathbuf();
            cache_path.push(COOKIE_STORE_PATH);

            if cache_path.is_file() {
                File::open(cache_path).map(BufWriter::new).unwrap()
            } else {
                File::create(cache_path).map(BufWriter::new).unwrap()
            }
        };

        let store = self.cookies.lock().unwrap();
        store.save_json(&mut writer).unwrap();
    }
}

impl Drop for DocumentCache {
    fn drop(&mut self) {
        self.save_cookies();
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
            Some(l) => create_cache_dir(l.as_str()).with_context(|| "Error building document cache").map(DocumentCache::new),
            None => create_cache_dir(get_runtime_path().expect("Could not determine XDG_RUNTIME_DIR").as_str()).map(DocumentCache ::new)
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