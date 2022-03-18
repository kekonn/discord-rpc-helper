mod steam;

use std::process::exit;

use sysinfo::{Process, ProcessExt, System, SystemExt};
use steam::SteamProcess;

/// Returns true if the process was started with wine64-preloader
fn filter_process(proc: &Process) -> bool {
    proc.name().to_lowercase() == "reaper"
}

fn main() {
    if !System::IS_SUPPORTED {
        println!("sysinfo library is not supported on this platform. Exiting...");
        exit(-1);
    }

    let mut sys = System::new_all();

    sys.refresh_processes();

    for (_, process) in sys.processes().iter().filter(|p| filter_process(p.1)) {
            println!(
                "Process: {} - Steam AppId: {}\n\tPath: {}",
                process.name(),
                process.steam_appid(),
                match process.steam_path().unwrap() {
                    None => "",
                    Some(path) => path
                }
            );
    }
}
