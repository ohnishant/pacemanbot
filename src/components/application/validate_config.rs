use serenity::{
    client::Context,
    model::{id::GuildId, prelude::application_command::ApplicationCommandInteraction},
};

use crate::{cache::guild_data::GuildData, Result};

pub async fn validate_config(
    ctx: &Context,
    guild_id: GuildId,
    command: &ApplicationCommandInteraction,
) -> Result<()> {
    command.defer_ephemeral(&ctx).await?;
    let reply_content;
    match GuildData::new(&ctx, guild_id).await {
        Ok(_) => {
            reply_content =
                "Config validation successful! Bot will send paces in #pacemanbot.".to_string()
        }
        Err(err) => reply_content = format!("Error: {}", err),
    };
    command
        .edit_original_interaction_response(&ctx, |m| m.content(reply_content))
        .await?;
    Ok(())
}