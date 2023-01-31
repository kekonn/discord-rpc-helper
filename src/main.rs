mod config;
mod steam;
mod constants;

use anyhow::{ anyhow, bail, Result };
use config::Configuration;
use std::{ borrow::BorrowMut, time::Duration };
use steam::{ scanner::get_running_steam_games, SteamApp };
use tokio::{ signal, sync::broadcast::{ self, Receiver } };
use discord_sdk::{
    Discord,
    DiscordApp,
    Subscriptions,
    wheel::Wheel,
    activity::{ ActivityBuilder }
};
use tracing::{debug, info, error, event, Level};

#[tokio::main]
async fn main() -> Result<()> {
    let (shutdown_send, mut shutdown_recv) = broadcast::channel(5);
    tracing_subscriber::fmt::init();

    info!("Reading config.json");
    let config = match Configuration::from_file("config.json") {
        Ok(c) => c,
        Err(e) => {
            error!("Error loading configuration: {e:?}");
            return Err(e);
        }
    };

    match validate_config(&config) {
        Ok(()) => (),
        Err(errors) => {
            error!("Error loading configuration: {errors:?}");
            return Err(errors);
        }
    }

    debug!("Found client id {}", config.discord_client_id);

    tokio::spawn(async move {
        let loop_result = detection_loop(shutdown_recv.borrow_mut(), config.clone()).await;
        match loop_result {
            Ok (_) => (),
            Err(e) if format!("{e:#?}") == "ChannelDisconnected" => {
                debug!("Ignoring error about disconnected channel: {e:?}");
            },
            Err(e) => {
                error!("{e:#?}");
            }
        }
    });

    match signal::ctrl_c().await {
        Ok(_) => {
            info!("Received shutdown event. Sending shutdown signals (can take up to 1 minute)");
            shutdown_send.send(())?;
        }
        Err(e) => bail!("Error catching Ctrl-C signal: {}", e),
    }

    Ok(())
}

async fn detection_loop(shutdown_recv: &mut Receiver<()>, config: Configuration) -> Result<()> {
    let (wheel, handler) = Wheel::new(
        Box::new(|err| {
            error!("Discord SDK error: {:?}", err);
        })
    );
    let discord = Discord::new(
        DiscordApp::PlainId(config.discord_client_id.parse()?),
        Subscriptions::ACTIVITY,
        Box::new(handler)
    )?;

    let mut user = wheel.user();

    info!("Waiting for handshake from Discord SDK");
    user.0.changed().await?;
    info!("Connected to Discord");

    let sleep_dur = Duration::from_secs(10);
    let mut running_id = constants::NO_APPID;

    event!(Level::INFO, "Starting to monitor for Steam games...");

    loop {
        let running_games = get_games()?;

        match running_games.len() {
            0 if running_id != constants::NO_APPID => {
                event!(Level::INFO, "Game no longer running. Clearing activity...");
                running_id = discord.clear_activity().await.map(|_| constants::NO_APPID)?;
            }
            0 if running_id == constants::NO_APPID => {}
            _ => {
                let game = &running_games[0];

                if running_id != game.app_id {
                    let game_name = game.get_name().await?;
                    event!(Level::INFO, "Setting activity to game {}", &game_name);

                    running_id = discord
                        .update_activity(
                            ActivityBuilder::default()
                            .start_timestamp(game.running_since)
                            .details(format!("Playing {game_name:?}"))
                        )
                        .await
                        .map(|res| {
                            if res.is_some() {
                                game.app_id
                            } else {
                                error!("Error setting activity");
                                constants::NO_APPID
                            }
                        })?;
                }
            }
        }

        tokio::select! {
            biased;
            _ = tokio::time::sleep(sleep_dur) => {},
            _ = shutdown_recv.recv() => {
                info!("Shutting down and clearing activity");
                _ = discord.clear_activity().await?;
                break
            }
        }
    }

    Ok(())
}

fn get_games() -> Result<Vec<SteamApp>> {
    match get_running_steam_games() {
        Ok(games) => Ok(games),
        Err(err) => Err(anyhow!("Error trying to find steam games: {}", err)),
    }
}

fn validate_config(config: &Configuration) -> Result<()> {
    let validation_result = config.validate();

    if validation_result.is_empty() {
        return Ok(());
    }

    let err_msg = validation_result
        .iter()
        .fold(String::from("Error messages:"), |acc, x| { format!("{acc:?}\n\t- {x:?}") });

    Err(anyhow!(err_msg))
}