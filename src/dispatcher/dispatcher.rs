use crate::{utils::get_event_type::get_event_type, ws::response::EventType, Result};

use super::{
    consts::SPECIAL_UNDERSCORE, non_pace_event::handle_non_pace_event,
    pace_event::handle_pace_event, Dispatcher,
};

impl Dispatcher {
    pub async fn dispatch(&self) -> Result<()> {
        let last_event = match self.response.event_list.last() {
            Some(evt) => evt,
            None => {
                return Err(format!(
                    "DispatcherError: get last event from events list for response: {:#?}",
                    self.response
                )
                .into())
            }
        };
        let mut locked_guild_cache = self.cache_manager.lock().await;
        for (_, guild_data) in locked_guild_cache.cache.iter_mut() {
            let live_link = match self.response.user.live_account.to_owned() {
                Some(acc) => format!(
                    "[{}](<https://twitch.tv/{}>)",
                    self.response.nickname.replace("_", SPECIAL_UNDERSCORE),
                    acc
                ),
                None => {
                    if !guild_data.is_private {
                        println!(
                            "Skipping guild: '{}' because user with name: '{}' is not live.",
                            guild_data.name, self.response.nickname,
                        );
                        continue;
                    }
                    format!(
                        "Offline - {}",
                        self.response.nickname.replace("_", SPECIAL_UNDERSCORE)
                    )
                }
            };
            let event_type = match get_event_type(&last_event) {
                Some(etype) => etype,
                None => {
                    return Err(format!(
                        "DispatcherError: get event type for event: {:#?}. Skipping all guilds.",
                        last_event.event_id,
                    )
                    .into());
                }
            };
            match event_type {
                EventType::NonPaceEvent => {
                    handle_non_pace_event(
                        self.ctx.clone(),
                        &self.response,
                        live_link,
                        last_event,
                        guild_data,
                    )
                    .await;
                }
                EventType::PaceEvent => {
                    handle_pace_event(
                        self.ctx.clone(),
                        &self.response,
                        live_link,
                        last_event,
                        guild_data,
                    )
                    .await;
                }
            }
        }
        Ok(())
    }
}
