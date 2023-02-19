use super::{*, cache::DocumentCacheBuilder};
use sysinfo::{Process, ProcessExt, RefreshKind, System, SystemExt};
use anyhow::Result;
use tracing::debug;

/// Returns true if the process was started with wine64-preloader
fn filter_process(proc: &Process) -> bool {
    proc.name().to_lowercase() == "reaper"
}

fn process_to_steamapp(steamproc: &Process) -> Option<SteamApp> {
    let path = steamproc.steam_path()
        .unwrap_or(None)
        .unwrap();
    
    let cache = DocumentCacheBuilder::new().build().expect("Error creating the document cache");

    let app = SteamApp {
        app_id: steamproc.steam_appid(),
        path,
        running_since: steamproc.start_time() as i64,
        exe_name: String::from(steamproc.exe().to_str().unwrap_or("")),
        cache
    };

    debug!("Found the following app: {:#?}", app);

    Some(app)
}

/// Gets all running steam games
pub fn get_running_steam_games() -> Result<Vec<SteamApp>, &'static str> {
    let mut sys = System::new_with_specifics(RefreshKind::everything());

    sys.refresh_processes();

    let apps: Vec<SteamApp> = sys
        .processes()
        .iter()
        .filter(|(_, p)| filter_process(p))
        .map_while(|(_, p)| process_to_steamapp(p))
        .collect();

    Ok(apps)
}