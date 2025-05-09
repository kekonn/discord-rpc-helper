use super::{*};
use sysinfo::{Process, ProcessesToUpdate, RefreshKind, System};
use anyhow::Result;

/// Returns true if the process was started with wine64-preloader
fn filter_process(proc: &Process) -> bool {
    proc.name().eq_ignore_ascii_case("reaper")
}

fn process_to_steamapp(steamproc: &Process) -> Option<SteamApp> {
    let path = steamproc.steam_path()
        .unwrap_or(None);

    path.as_ref()?;

    Some(SteamApp {
        app_id: steamproc.steam_appid(),
        path: path.unwrap(),
        running_since: steamproc.start_time() as i64,
    })
}

/// Gets all running steam games
pub fn get_running_steam_games() -> Result<Vec<SteamApp>, &'static str> {
    let mut sys = System::new_with_specifics(RefreshKind::everything());

    sys.refresh_processes(ProcessesToUpdate::All, true);

    let apps: Vec<SteamApp> = sys
        .processes()
        .iter()
        .filter(|(_, p)| filter_process(p))
        .map_while(|(_, p)| process_to_steamapp(p))
        .collect();

    Ok(apps)
}