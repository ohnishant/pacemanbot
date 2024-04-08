use crate::handler_utils::*;
use serenity::{
    async_trait,
    client::{Context, EventHandler},
    model::{application::interaction::Interaction, gateway::Ready, prelude::Guild},
};
use std::sync::Arc;

pub struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn guild_create(&self, ctx: Context, guild: Guild, _is_new: bool) {
        handle_guild_create(&ctx, guild.id).await;
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        handle_interaction_create(&ctx, interaction).await;
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
        ctx.cache.set_max_messages(100);
        let ctx = Arc::new(ctx);
        tokio::spawn(async move { handle_ready(ctx.clone()).await });
    }
}
