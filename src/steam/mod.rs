mod cache;

pub mod scanner;

use anyhow::{anyhow, Result};
use constants::{APPID_ENV_KEY, NO_APPID, STEAM_GAME_PATH_FRAGMENT};
use once_cell::sync::OnceCell;
use sysinfo::{Process};
use self::cache::DocumentCache;

use super::constants;

/// Describes functionalities of a Steam Proton process
trait SteamProcess {
    /// Returns the Steam game's AppId
    fn steam_appid(&self) -> u32;
    /// Returns the path of the game executable
    fn steam_path(&self) -> Result<Option<String>>;
}

impl SteamProcess for Process {
    fn steam_appid(&self) -> u32 {
        let appid_environ = self.environ()
            .iter().filter_map(|e| e.to_str())
            .find(|&e| e.starts_with(APPID_ENV_KEY));

        match appid_environ {
            Some(id) => id.split('=').last().unwrap().parse::<u32>().unwrap(),
            None => NO_APPID,
        }
    }

    fn steam_path(&self) -> Result<Option<String>> {
        let filtered: Vec<&str> = self
            .cmd()
            .iter().filter_map(|e| e.to_str())
            .filter_map(|c| match c.contains(STEAM_GAME_PATH_FRAGMENT) && c.ends_with(".exe") {
                true => Some(c),
                false => None,
            })
            .collect();

        match filtered.len() {
            0 => Ok(None),
            1 => Ok(Some(filtered[0].to_owned())),
            _ => Err(anyhow!("Found multiple possible paths for process '{:?}'", self.name()))
        }
    }
}

static CACHE: OnceCell<DocumentCache> = OnceCell::new();

fn get_cache() -> &'static DocumentCache {
    CACHE.get_or_init(|| cache::DocumentCacheBuilder::new().build().expect("Error creating the document cache"))
}

#[derive(Debug, PartialEq, Eq)]
pub struct SteamApp {
    pub app_id: u32,
    pub path: String,
    pub running_since: i64,
}

impl SteamApp {
    #[allow(dead_code)]
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
    pub async fn get_name(&self) -> Result<String> {
        let steam_url = self.get_steam_url();
        get_cache().get_name(steam_url.as_str()).await
    }

    #[allow(dead_code)]
    /// Gets the url to the game's icon
    pub async fn get_app_icon_url(&self) -> Result<String> {
        let steam_url = self.get_steam_url();
        get_cache().get_appicon(steam_url.as_str()).await
    }
}

#[cfg(test)]
mod tests {

    use super::SteamApp;
    use anyhow::Result;

    #[test]
    fn steamapp_renders_store_url() -> Result<()> {
        let app = SteamApp {
            app_id: 1,
            path: String::from(""),
            running_since: 18,
        };

        let store_url = app.get_steam_url();

        assert_eq!(store_url, "https://store.steampowered.com/app/1/");

        Ok(())
    }
}
