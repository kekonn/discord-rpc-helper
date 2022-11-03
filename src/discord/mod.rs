pub mod discord_presence;

use super::steam::*;
use anyhow::{Result};
use std::time::{Duration, SystemTime};
use async_trait::async_trait;


pub struct Client  {
    pub client_id: String,
    pres_client: Option<discord_rich_presence::DiscordIpcClient>
}

#[async_trait]
pub trait DiscordClient {
    async fn new (client_id: &str) -> Result<Client>;
    async fn clear_activity(&mut self) -> Result<()>;
    async fn set_activity(&mut self, game: &SteamApp) -> Result<()>;
    async fn check_connection(&mut self) -> Result<()>;
}

#[allow(dead_code)]
fn to_timestamp(dur: Duration, ref_time: Option<SystemTime>) -> SystemTime {
    ref_time.unwrap_or_else(SystemTime::now) - dur
}
