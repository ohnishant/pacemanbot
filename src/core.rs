use std::{collections::HashMap, sync::Arc};

use serenity::{prelude::{Context, Mentionable}, model::id::GuildId};

use crate::{
    types::{Response, ArcMux, GuildData, Split, PlayerData},
    utils::{format_time, get_time, update_leaderboard},
};


pub async fn parse_record(ctx: Arc<Context>, record: Response, guild_cache: ArcMux<HashMap<GuildId, GuildData>>) {
    for guild_id in ctx.cache.guilds() {
        let mut locked_guild_cache = guild_cache.lock().await;
        let locked_guild_cache = match locked_guild_cache.get_mut(&guild_id) {
            Some(cache) => cache,
            None => {
                let guild_data = match GuildData::new(&ctx, guild_id.to_owned()).await {
                    Ok(data) => data,
                    Err(err) => {
                        eprintln!("{}", err);
                        continue;
                    }
                };
                locked_guild_cache.insert(guild_id.to_owned(), guild_data);
                locked_guild_cache.get_mut(&guild_id).unwrap()
            }
        };
        let guild_name = locked_guild_cache.name.to_owned();
        let channel_to_send_to = locked_guild_cache.pace_channel;
        let guild_roles = &locked_guild_cache.roles;

        let player_data = match locked_guild_cache.players.get_mut(&record.nickname.to_lowercase()) {
            Some(data) => data,
            None => {
                if locked_guild_cache.is_private {
                     println!(
                         "Skipping because player, with name '{}', is not in the runners channel for guild name: '{}'.", 
                          record.nickname.to_owned(),
                          guild_name
                      );
                     continue;
                }
                let player_data = PlayerData::new();
                locked_guild_cache.players.insert(record.nickname.to_owned(), player_data);
                locked_guild_cache.players.get_mut(&record.nickname.to_owned()).unwrap()
            }
        };
        let player_splits = &player_data.splits;

        let last_event = match record.event_list.last() {
            Some(event) => event.to_owned(),
            None => {
                eprintln!("No events in event list for record: {:#?}.", record);
                continue;
            }
        };
        
        let split: Split;
        let split_desc: String;
        let mut bastionless_content: &str = "";
        match last_event.event_id.as_str() {
            "common.open_to_lan"| "common.multiplayer" | "common.enable_cheats" | "common.view_seed" | "common.leave_world" => {
                let message_id = match player_data.last_pace_message {
                    Some(id) => id,
                    None => {
                        eprintln!("No last pace message to edit for reset.");
                        continue;
                    }
                };
                let mut message = match ctx.cache.message(channel_to_send_to, message_id) {
                    Some(message) => message,
                    None => {
                        eprintln!("Unable to construct message from message id: {}", message_id);
                        continue;
                    }
                };
                let prev_content = message.content.split('\n').collect::<Vec<&str>>();
                let first_line = match prev_content.first() {
                    Some(line) => line,
                    None => {
                        eprintln!("Unable to get first line from message with id: {}", message_id);
                        continue;
                    }
                };
                let new_first_line = format!("{} (Reset)", first_line);
                let other_lines = 
                    prev_content.iter().filter(|l| l.to_owned() != first_line).map(|l| l.to_owned()).collect::<Vec<&str>>().join("\n");
                let new_content = format!("{}\n{}", new_first_line, other_lines);
                match message.edit(&ctx.http, |m| m.content(new_content)).await {
                    Ok(_)=> (),
                    Err(err) => 
                        eprintln!("Unable to edit message with id: {} due to: {}", message_id, err),
                };
                continue;
            }
            "rsg.credits" => {
                let runner_name = record.nickname.to_owned();
                let (minutes, seconds) = get_time(last_event.igt as u64);
                match update_leaderboard(&ctx, &guild_id, runner_name.to_owned(), (minutes, seconds))
                    .await
                {
                    Ok(_) => {
                        println!(
                            "Updated leaderboard in #pacemanbot-runner-leaderboard for guild name: {}, runner name: {} with time: {}.", 
                            guild_name, 
                            runner_name, 
                            format_time(last_event.igt as u64),
                        );
                        continue;
                    }
                    Err(err) => {
                        eprintln!(
                            "Unable to update leaderboard in guild name: {} for runner name: {} due to: {}",
                            guild_name, 
                            record.nickname.to_owned(), 
                            err
                        );
                        continue;
                    }
                };
            }
            "rsg.enter_bastion" => {
                if record.context_event_list
                    .iter()
                    .any(|ctx| ctx.event_id == "rsg.obtain_blaze_rod") {
                    split = Split::SecondStructure;
                } else {
                    split = Split::FirstStructure;
                }
                split_desc = split.desc(Some("Bastion"));
            }
            "rsg.enter_fortress" => {
                let fort_ss_check = record
                    .event_list
                    .iter()
                    .filter(|evt| evt != &last_event)
                    .any(|evt| evt.event_id == "rsg.enter_bastion");

                let mut fort_ss_context_check = false;
                let mut context_hits = 0;
                for ctx in record.context_event_list.iter() {
                    let context_check = ctx.event_id == "rsg.obtain_crying_obsidian" 
                        || ctx.event_id == "rsg.obtain_obsidian" 
                        || ctx.event_id == "rsg.loot_bastion";
                    if context_check {
                        context_hits += 1;
                    } 
                }
                if context_hits >= 2 {
                    fort_ss_context_check = true;
                }

                if fort_ss_check && fort_ss_context_check {
                    split = Split::SecondStructure
                } else {
                    split = Split::FirstStructure
                }
                split_desc = split.desc(Some("Fortress"));
            }
            "rsg.first_portal" => {
                if !record
                    .event_list
                    .iter()
                    .filter(|evt| evt != &last_event)
                    .any(|evt| evt.event_id == "rsg.enter_bastion")
                {
                    bastionless_content = "(Bastionless)";
                }
                split = match Split::from_event_id(last_event.event_id.as_str()) {
                    Some(split) => split,
                    None => {
                        eprintln!("Unable to convert event id: 'rsg.first_portal' to split.");
                        continue;
                    }
                };
                split_desc = split.desc(None);
            }
            _ => {
                if Split::from_event_id(last_event.event_id.as_str()).is_none() {
                    if last_event.event_id.as_str() == "rsg.credits" {
                        println!(
                            "Skipping guild with name '{}' for event id: '{}'.", 
                            guild_name, 
                            last_event.event_id
                        );
                        // Check other guilds here because we would want to check all guilds for a
                        // completion.
                        continue;
                    }
                    println!(
                        "Skipping event id: '{}' as it is unrecognized.",
                        last_event.event_id
                    );
                    // Skip checking other guilds as the event id will not be recognized in them as
                    // well.
                    return;
                }
                split = match Split::from_event_id(last_event.event_id.as_str()) {
                    Some(split) => split,
                    None => {
                        eprintln!("Unable to convert event id: '{}' to split.", last_event.event_id);
                        continue;
                    }
                };
                split_desc = split.desc(None);
            }
        }

        let roles_to_ping = guild_roles
            .iter()
            .filter(|role| {
                let (split_minutes, split_seconds) = get_time(last_event.igt as u64);
                if role.guild_role.name.contains("PB") {
                    if !locked_guild_cache.is_private {
                        return false;
                    }
                    let pb_minutes = player_splits.get(&role.split).unwrap().to_owned();
                    role.split == split && pb_minutes > split_minutes
                } else {
                    role.split == split
                        && role.minutes >= split_minutes
                        && (role.minutes != split_minutes || role.seconds > split_seconds)
                }
            })
            .collect::<Vec<_>>();

        let live_link = match record.user.live_account.to_owned() {
            Some(acc) => format!("[{}](<https://twitch.tv/{}>)", record.nickname, acc),
            None => {
                if !locked_guild_cache.is_private {
                    println!(
                        "Skipping split: '{}' because user with name: '{}' is not live.",
                        split_desc, record.nickname,
                    );
                    continue;
                } else {
                    format!("Offline - {}", record.nickname.to_owned())
                }
            }
        };

        if roles_to_ping.is_empty() {
            println!(
                "Skipping split: '{}' because there are no roles to ping in guild name: {}.",
                split_desc, guild_name
            );
            continue;
        }

        let content = format!(
            "## {} - {} {}\n{}\t<t:{}:R>\n{}",
            format_time(last_event.igt as u64),
            split_desc,
            bastionless_content,
            live_link,
            (record.last_updated / 1000) as u64,
            roles_to_ping
                .iter()
                .map(|role| role.guild_role.mention().to_string())
                .collect::<Vec<_>>()
                .join(" "),
        );

        player_data.last_pace_message = match channel_to_send_to
            .send_message(&ctx, |m| m.content(content))
            .await
        {
            Ok(message) => {
                println!(
                    "Sent pace-ping for user with name: '{}' for split: '{}' in guild name: {}.",
                    record.nickname, split_desc, guild_name
                );
                player_data.last_split = Some(split);
                Some(message.id)
            }
            Err(err) => {
                eprintln!(
                    "Unable to send split: '{}' with roles: {:?} due to: {}",
                    split_desc, roles_to_ping, err
                );
                continue;
            }
        };
    }
}
