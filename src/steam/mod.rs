mod cache;

pub mod scanner;

use anyhow::{anyhow, Result};
use constants::{APPID_ENV_KEY, NO_APPID, STEAM_GAME_PATH_FRAGMENT};
use sysinfo::{Process, ProcessExt};
use self::cache::DocumentCache;

use super::constants;

/// Describes functionalities of a Steam Proton process
trait SteamProcess: ProcessExt {
    /// Returns the Steam game's AppId
    fn steam_appid(&self) -> u32;
    /// Returns the path of the game executable
    fn steam_path(&self) -> Result<Option<String>>;
}

impl SteamProcess for Process {
    fn steam_appid(&self) -> u32 {
        let appid_environ = self.environ().iter().find(|e| e.starts_with(APPID_ENV_KEY));

        match appid_environ {
            Some(id) => id.split("=").last().unwrap().parse::<u32>().unwrap(),
            None => NO_APPID,
        }
    }

    fn steam_path(&self) -> Result<Option<String>> {
        let filtered: Vec<&str> = self
            .cmd()
            .into_iter()
            .filter_map(|c| match c.contains(STEAM_GAME_PATH_FRAGMENT) && c.ends_with(".exe") {
                true => Some(c.as_str()),
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

#[derive(Debug, PartialEq, Eq)]
pub struct SteamApp {
    pub app_id: u32,
    pub path: String,
    pub running_since: u64,
    cache: DocumentCache
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
    pub async fn get_name(&self) -> Result<String> {
        let steam_url = self.get_steam_url();
        self.cache.get_name(steam_url.as_str()).await
    }

    /// Gets the url to the game's icon
    pub async fn get_app_icon_url(&self) -> Result<String> {
        let steam_url = self.get_steam_url();
        self.cache.get_appicon(steam_url.as_str()).await
    }
}

#[cfg(test)]
mod tests {

    use crate::steam::cache::DocumentCacheBuilder;
    use super::SteamApp;
    use anyhow::Result;

    #[test]
    fn steamapp_renders_store_url() -> Result<()> {
        let cache = DocumentCacheBuilder::new().build()?;

        let app = SteamApp {
            app_id: 1,
            path: String::from(""),
            running_since: 18,
            cache
        };

        let store_url = app.get_steam_url();

        assert_eq!(store_url, "https://store.steampowered.com/app/1/");

        Ok(())
    }

    #[test]
    fn steamapp_renders_steamdb_url() -> Result<()> {
        let cache = DocumentCacheBuilder::new().build()?;

        let app = SteamApp {
            app_id: 1,
            path: String::from(""),
            running_since: 18,
            cache
        };

        let steamdb_url = app.get_steamdb_url();

        assert_eq!(steamdb_url, "https://steamdb.info/app/1/");

        Ok(())
    }
}
