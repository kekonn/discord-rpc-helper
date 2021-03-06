mod steam;
mod discord;
mod config;
pub mod constants;

use std::{borrow::BorrowMut, time::Duration};
use steam::{scanner::get_running_steam_games, SteamApp};
use config::Configuration;
use anyhow::{Result, anyhow, bail};
use tokio::{signal, sync::broadcast::{self, Receiver}};
use crate::discord::{Client, DiscordClient};

#[tokio::main]
async fn main() -> Result<()> {
    let (shutdown_send, mut shutdown_recv) = broadcast::channel(5);

    println!("Reading config.json");
    let config = Configuration::from_file("config.json").unwrap();

    println!("Found client id {}", config.discord_client_id);

    println!("Starting to monitor for Steam games...");

    tokio::spawn(async move {
        detection_loop(shutdown_recv.borrow_mut(), config.clone()).await.unwrap();
    });

    match signal::ctrl_c().await {
        Ok(_) => {
            shutdown_send.send(())?;
        }
        Err(e) => bail!("Error catching Ctrl-C signal: {}", e),
    };

    Ok(())
}

async fn detection_loop(shutdown_recv: &mut Receiver<()>, config: Configuration) -> Result<()> {
    let mut client_result = Client::new(&config.discord_client_id).await;
    let mut client: Client;
    let long_sleep = Duration::from_secs(60);

    // Wait for connection before starting game detection
    loop {
        match client_result {
            Ok(c) => {
                client = c;
                break;
            },
            Err(e) => {
                println!("{}. Retrying in 1 minute", e);
                tokio::time::sleep(long_sleep).await;
                client_result = Client::new(&config.discord_client_id).await;
            },
        };
    }

    let sleep_dur = Duration::from_secs(10);

    let mut running_id = constants::NO_APPID;
    loop {

        // Check connection before setting game activity
        // Not important when first entering the loop, but discord could be closed in between the first check and setting activity
        match client.check_connection().await {
            Err(e) => {
                println!("Connection check failed: {}. Trying again in 1 minute", e);
                running_id = constants::NO_APPID;
                tokio::time::sleep(long_sleep).await;
                continue;
            },
            _ => (),
        }

        let running_games = get_games()?;

        match running_games.len() {
            0 if running_id != constants::NO_APPID => {
                println!("Game no longer running. Clearing activity...");
                running_id = match clear_activity(&mut client).await {
                    Ok(_) => constants::NO_APPID,
                    Err(e) => {
                        println!("Error clearing activity: {}", e);
                        constants::NO_APPID
                    },
                };
            }
            0 if running_id == constants::NO_APPID => {}
            _ => {
                let game = &running_games[0];
                if running_id != game.app_id {
                    running_id = match set_activity(&mut client, game).await {
                        Ok(_) => game.app_id,
                        Err(e) => {
                            println!("Error setting activity: {}", e);
                            constants::NO_APPID
                        },
                    };
                }
            }
        };

        tokio::select! {
            _ = tokio::time::sleep(sleep_dur) => {},
            _ = shutdown_recv.recv() => {
                println!("Shutting down and clearing activity");
                _ = clear_activity(&mut client).await;
                break
            },
        };
    }

    Ok(())
}

async fn clear_activity(client: &mut Client) -> Result<()> {
    client.clear_activity().await
}

async fn set_activity(client: &mut Client, game: &SteamApp) -> Result<()> {
    println!("Found game {} ({}). Setting activity...", game.get_name().await?, game.app_id);
    client.set_activity(&game).await
}

fn get_games() -> Result<Vec<SteamApp>> {
    match get_running_steam_games() {
        Ok(games) => Ok(games),
        Err(err) => Err(anyhow!("Error trying to find steam games: {}", err)),
    }
}