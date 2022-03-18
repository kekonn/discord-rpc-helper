#[path = "constants.rs"]
mod constants;

use constants::{APPID_ENV_KEY, NO_APPID, STEAM_GAME_PATH_FRAGMENT};
use sysinfo::{Process, ProcessExt};

/// Describes functionalities of a Steam Proton process
pub trait SteamProcess: ProcessExt {
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
            0 => Err("Could not find a probable Steam game path"),
            1 => Ok(Some(filtered[0])),
            _ => Err("Found more than 1 possible path"),
        }
    }
}
