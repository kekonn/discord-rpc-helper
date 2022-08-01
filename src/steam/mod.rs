#[path = "../constants.rs"]
mod constants;
mod cache;

pub mod scanner;

use anyhow::anyhow;
use constants::{APPID_ENV_KEY, NO_APPID, STEAM_GAME_PATH_FRAGMENT};
use scraper::{ElementRef, Html, Selector};
use sysinfo::{Process, ProcessExt};

const STEAM_NAME_SELECTOR: &str = "#appHubAppName";
const STEAM_ICON_SELECTOR: &str = "div.apphub_AppIcon img";

/// Describes functionalities of a Steam Proton process
trait SteamProcess: ProcessExt {
    /// Returns the Steam game's AppId
    fn steam_appid(&self) -> u32;
    /// Returns the path of the game executable
    fn steam_path(&self) -> Result<Option<&String>, &'static str>;
}

impl SteamProcess for Process {
    fn steam_appid(&self) -> u32 {
        let appid_environ = self.environ().iter().find(|e| e.starts_with(APPID_ENV_KEY));

        match appid_environ {
            Some(id) => id.split("=").last().unwrap().parse::<u32>().unwrap(),
            None => NO_APPID,
        }
    }

    fn steam_path(&self) -> Result<Option<&String>, &'static str> {
        let filtered: Vec<&String> = self
            .cmd()
            .iter()
            .filter(|c| c.contains(STEAM_GAME_PATH_FRAGMENT) && c.ends_with(".exe"))
            .collect();

        let len = filtered.len();

        match len {
            0 => Ok(None),
            1 => Ok(Some(filtered[0])),
            _ => Err("Found more than 1 possible path"),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct SteamApp {
    pub app_id: u32,
    pub path: String,
    pub running_since: u64,
}

impl SteamApp {
    #[allow(dead_code)]
    /// Gets the SteamDb Url for the game
    pub fn get_steamdb_url(&self) -> String {
        format!("https://steamdb.info/app/{}/", self.app_id)
    }

    /// Gets the url to the poster image of the game
    pub fn get_large_poster_url(&self) -> String {
        format!(
            "https://cdn.cloudflare.steamstatic.com/steam/apps/{}/library_600x900_x2.jpg",
            self.app_id
        )
    }

    /// Gets the steam url to the games' store page
    pub fn get_steam_url(&self) -> String {
        format!("https://store.steampowered.com/app/{}/", self.app_id)
    }

    /// Try to resolve the game's name by scraping the store page
    pub async fn get_name(&self) -> anyhow::Result<String> {
        let steam_url = self.get_steam_url();
        get_name_cached(steam_url.as_str()).await
    }

    /// Gets the url to the game's icon
    pub async fn get_app_icon_url(&self) -> anyhow::Result<String> {
        let steam_url = self.get_steam_url();
        get_appicon_cached(steam_url.as_str()).await
    }
}

async fn get_name_cached(steam_url: &str) -> anyhow::Result<String> {
    let name_selector = Selector::parse(STEAM_NAME_SELECTOR).unwrap();
    let html = match download_steamdb_page(&steam_url).await {
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

async fn get_appicon_cached(steam_url: &str) -> anyhow::Result<String> {
    let img_selector = Selector::parse(STEAM_ICON_SELECTOR).unwrap();
    let html = match download_steamdb_page(&steam_url).await {
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

fn get_html(html: &str) -> Html {
    Html::parse_document(html)
}

async fn download_steamdb_page(url: &str) -> anyhow::Result<String> {
    let document = match reqwest::get(url).await {
        Ok(r) => r.text().await.unwrap(),
        Err(err) => return Err(anyhow!(err)),
    };

    Ok(document)
}

#[cfg(test)]
mod tests {

    use super::SteamApp;

    #[test]
    fn steamapp_renders_store_url() {
        let app = SteamApp {
            app_id: 1,
            path: String::from(""),
            running_since: 18,
        };

        let store_url = app.get_steam_url();

        assert_eq!(store_url, "https://store.steampowered.com/app/1/");
    }

    #[test]
    fn steamapp_renders_steamdb_url() {
        let app = SteamApp {
            app_id: 1,
            path: String::from(""),
            running_since: 18,
        };

        let steamdb_url = app.get_steamdb_url();

        assert_eq!(steamdb_url, "https://steamdb.info/app/1/");
    }
}
