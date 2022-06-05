use super::{super::steam::*, Client, DiscordClient};
use anyhow::{anyhow, bail, Result};
use async_trait::async_trait;
use discord_rich_presence::{
    activity::{self, Assets, Timestamps},
    DiscordIpc, DiscordIpcClient,
};
use std::time::Duration;

#[async_trait]
impl DiscordClient for Client {
    ///  `client_id`: Enter the client id from your registered Discord App
    async fn new(client_id: &str) -> Result<Client> {
        println!("Using discord_rich_presence client");

        if client_id.is_empty() {
            bail!(r#"Invalid client id: client id is empty"#)
        }

        let mut client = match DiscordIpcClient::new(client_id) {
            Ok(c) => c,
            Err(e) => return Err(anyhow!("Error creating client: {}", e)),
        };

        match client.connect() {
            Ok(_) => Ok(Self {
                client_id: client_id.to_owned(),
                pres_client: Some(client),
                rpc_client: None,
            }),
            Err(e) => Err(anyhow!("Error connecting to Discord: {}", e)),
        }
    }

    /// Clear all set activity data.
    async fn clear_activity(&mut self) -> Result<()> {
        let client = match &mut self.pres_client {
            Some(c) => c,
            None => return Err(anyhow!("You are trying to use the wrong api")),
        };

        match client.close() {
            Ok(_) => Ok(()),
            Err(e) => Err(anyhow!("Error closing client: {}", e)),
        }
    }

    async fn set_activity(&mut self, game: &SteamApp) -> Result<()> {
        let client = match &mut self.pres_client {
            Some(c) => c,
            None => return Err(anyhow!("You are trying to use the wrong api")),
        };

        client.reconnect().unwrap();

        let game_name = game.get_name().await?;
        let icon_url = game.get_app_icon_url().await?;
        let poster_url = game.get_large_poster_url();
        let running_dur = Duration::from_secs(game.running_since);

        match client.set_activity(
            activity::Activity::new()
                .state("Playing on Linux using Proton")
                .details(&game_name)
                .assets(
                    Assets::new()
                        .large_image(&poster_url)
                        .small_image(&icon_url),
                )
                .timestamps(Timestamps::new().start(running_dur.as_secs() as i64)),
        ) {
            Ok(_) => Ok(()),
            Err(e) => Err(anyhow!("Error trying to set activity: {}", e)),
        }
    }

    /// Tries to reconnect and will return `Ok(())` when successful or `Error` when it's not
    async fn check_connection(&mut self) -> Result<()> {
        let client = match &mut self.pres_client {
            Some(c) => c,
            None => return Err(anyhow!("You are trying to use the wrong api")),
        };

        match client.reconnect() {
            Ok(_) => Ok(()),
            Err(e) => Err(anyhow!("{}", e)),
        }
    }
}
