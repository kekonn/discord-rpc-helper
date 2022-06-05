use async_trait::async_trait;
use super::{super::steam::*, DiscordClient, Client, to_timestamp};
use anyhow::{anyhow, bail, Result};
use discord_sdk as ds;
use ds::activity::{ActivityBuilder, Assets};
use std::time::Duration;

#[async_trait]
impl DiscordClient for Client {
    ///  `client_id`: Enter the client id from your registered Discord App
    async fn new(client_id: &str) -> Result<Client> {
        println!("Using discord-sdk client");

        if client_id.is_empty() {
            bail!(r#"Invalid client id: client id is empty"#)
        }

        let app_id = match client_id.parse::<i64>() {
            Ok(id) => id,
            Err(e) => bail!(e),
        };

        let (wheel, handler) = ds::wheel::Wheel::new(Box::new(|err| {
            panic!("Discord client encountered an error: {}", err);
        }));

        let mut user = wheel.user();

        let rpc_client = match ds::Discord::new(
            ds::DiscordApp::PlainId(app_id),
            ds::Subscriptions::ACTIVITY,
            Box::new(handler),
        ) {
            Ok(d) => d,
            Err(e) => return Err(anyhow!(e)),
        };

        user.0.changed().await.unwrap();

        Ok(Self {
            client_id: client_id.to_owned(),
            rpc_client
        })
    }

    /// Clear all set activity data.
    async fn clear_activity(&self) -> Result<()> {
        self.rpc_client.clear_activity().await?;

        Ok(())
    }

    async fn set_activity(&self, game: &SteamApp) -> Result<()> {

        let game_name = game.get_name().await?;
        let icon_url = game.get_app_icon_url().await?;
        let poster_url = game.get_large_poster_url();
        let running_dur = Duration::from_secs(game.running_since);

        let payload = ActivityBuilder::default()
            .start_timestamp(to_timestamp(running_dur, None))
            .state("Playing on Linux using Proton")
            .details(&game_name)
            .assets(Assets::default()
                .large(poster_url, Some(&game_name))
                .small(icon_url, Some(&game_name)));

        match self.rpc_client.update_activity(payload).await {
            Ok(_) => Ok(()),
            Err(e) => Err(anyhow!("Error updating the presence: {}", e)),
        }
    }
}