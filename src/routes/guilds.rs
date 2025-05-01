/*
Copyright (C) 2025  BeTalky Community
This file is part of BeTalky.

BeTalky is free software: you can redistribute it and/or modify
it under the terms of the GNU Affero General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

BeTalky is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU Affero General Public License for more details.

You should have received a copy of the GNU Affero General Public License
along with BeTalky.  If not, see <https://www.gnu.org/licenses/>.
*/

use super::structs::{Channel, ChannelRole, CreateGuildBody, Guild, Member, PatchGuildBody, ReturnedGuild, Role};
use crate::{to_json_array, utils::{self, permissions::{check_guild_permission, GuildPermissions}}, AppError, Auth};

use rocket::{
    http::Status,
    serde::json::{from_value, serde_json, to_value, Json, Value},
    Route, State,
};
use std::{collections::HashMap, time::{SystemTime, UNIX_EPOCH}};
use uuid::Uuid;

#[get("/guilds/<guild_id>", format = "json")]
async fn get_guild(
    guild_id: &str,
    database: &State<tokio_postgres::Client>,
    user_id: Auth,
) -> Result<Json<ReturnedGuild>, AppError> {
    // Get guild
    let pre_guild = database
        .query_one(
            "SELECT * FROM guilds WHERE id = $1 AND EXISTS (
               SELECT 1
               FROM unnest(members) AS member
               WHERE member->>'id' = $2
           )",
            &[&Uuid::parse_str(guild_id).unwrap(), &user_id.0],
        )
        .await;

    if pre_guild.is_err() {
        return Err(AppError(Status::NotFound));
    }

    let guild = pre_guild.unwrap();

    Ok(Json(ReturnedGuild {
        id: guild.get::<&str, Uuid>("id").to_string(),
        name: guild.get::<&str, String>("name"),
        description: guild
            .try_get::<&str, Option<String>>("description")
            .unwrap_or(None),
        icon: guild
            .try_get::<&str, Option<String>>("icon")
            .unwrap_or(None),
        public: guild.get::<&str, bool>("public"),
        roles: serde_json::from_value(Value::Array(
            guild.get::<&str, Vec<Value>>("roles"),
        ))
        .unwrap(),
        members: guild
            .get::<&str, Vec<Value>>("members")
            .len(),
        creation: guild.get::<&str, i64>("creation"),
    }))
}

#[post("/guilds", format = "json", data = "<body>")]
async fn create_guild(
    body: Json<CreateGuildBody>,
    sse_clients: &State<crate::SSEClients>,
    database: &State<tokio_postgres::Client>,
    user_id: Auth,
) -> Result<Json<ReturnedGuild>, AppError> {
    // Check if name is too long
    if body.name.len() > 30 {
        return Err(AppError(Status::BadRequest));
    }

    // Create guild
    let guild = Guild {
        id: Uuid::new_v4().to_string(),
        name: body.name.clone(),
        description: body.description.clone(),
        icon: None,
        public: false,
        channels: vec![Channel {
            id: Uuid::new_v4().to_string(),
            name: "general".to_string(),
            topic: None,
            r#type: "text".to_string(),
            creation: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
            roles: vec![
                ChannelRole {
                    id: "00000000-0000-0000-0000-000000000000".to_string(),
                    permissions: 1, // TODO: Add owner permissions
                },
                ChannelRole {
                    id: "11111111-1111-1111-1111-111111111111".to_string(),
                    permissions: 0, // TODO: Add member permissions
                },
            ],
            messages: vec![],
            pins: vec![],
        }],
        roles: vec![
            Role {
                id: "00000000-0000-0000-0000-000000000000".to_string(),
                name: "Owner".to_string(),
                permissions: GuildPermissions::ADMINISTRATOR.bits(),
                color: None,
                hoist: false,
            },
            Role {
                id: "11111111-1111-1111-1111-111111111111".to_string(),
                name: "Members".to_string(),
                permissions: (GuildPermissions::CREATE_INVITE | GuildPermissions::CHANGE_NICKNAME).bits(),
                color: None,
                hoist: false,
            },
        ],
        members: vec![Member {
            id: user_id.0.clone(),
            nickname: None,
            roles: vec![
                "00000000-0000-0000-0000-000000000000".to_string(),
                "11111111-1111-1111-1111-111111111111".to_string(),
            ],
        }],
        creation: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64,
        bans: vec![],
        invites: vec![],
    };

    database.execute("INSERT INTO guilds (id, name, description, icon, public, channels, roles, members, creation, bans, invites) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
    &[
        &Uuid::parse_str(&guild.id).unwrap(),
        &guild.name,
        &guild.description,
        &guild.icon,
        &guild.public,
        to_json_array!(&guild.channels),
        to_json_array!(&guild.roles),
        to_json_array!(&guild.members),
        &guild.creation,
        to_json_array!(&guild.bans),
        to_json_array!(&guild.invites),
    ]).await?;

    let returned_guild = ReturnedGuild {
        id: guild.id.to_string(),
        name: guild.name,
        description: guild.description,
        icon: guild.icon,
        public: guild.public,
        roles: guild.roles,
        members: guild.members.len(),
        creation: guild.creation,
    };

    // Broadcast guildJoined event
    utils::sse::broadcast(
        sse_clients,
        &user_id.0,
        utils::structs::SSEEvent {
            event: "guildJoined",
            guild: Some(&returned_guild),
            ..Default::default()
        },
    )
    .await;

    Ok(Json(returned_guild))
}

#[patch("/guilds/<guild_id>", format = "json", data = "<body>")]
async fn update_guild(
    guild_id: &str,
    body: Json<PatchGuildBody>,
    sse_clients: &State<crate::SSEClients>,
    database: &State<tokio_postgres::Client>,
    user_id: Auth,
) -> Result<Json<ReturnedGuild>, AppError> {
    // Get guild
    let pre_guild = database
        .query_one(
            "SELECT * FROM guilds WHERE id = $1 AND EXISTS (
               SELECT 1
               FROM unnest(members) AS member
               WHERE member->>'id' = $2
           )",
            &[&Uuid::parse_str(guild_id).unwrap(), &user_id.0],
        ) 
        .await;

    if pre_guild.is_err() {
        return Err(AppError(Status::NotFound));
    }

    let mut guild = pre_guild.unwrap();

    // Get the current user
    let pre_me = database
        .query_one(
            "SELECT member FROM guilds,
            unnest(members) AS member
            WHERE id = $1
            AND member->>'id' = $2",
            &[&Uuid::parse_str(guild_id).unwrap(), &user_id.0],
        ) 
        .await;
    let mut me: Member = from_value(pre_me.unwrap().get("member")).unwrap();

    // Check if can manage the guild
    if !check_guild_permission(&guild, &me.id, GuildPermissions::MANAGE_GUILD) {
        return Err(AppError(Status::Forbidden));
    }

    // Transferring ownership
    if body.owner.is_some() {
        // Check if owner
        if !me.roles.contains(&"00000000-0000-0000-0000-000000000000".to_string()) {
            return Err(AppError(Status::Forbidden));
        }

        // Check if the new owner exists
        let pre_new_owner = database
        .query_one(
            "SELECT member FROM guilds,
            unnest(members) AS member
            WHERE id = $1
            AND member->>'id' = $2",
            &[&Uuid::parse_str(guild_id).unwrap(), &body.owner],
        ) 
        .await;

        if pre_new_owner.is_err() {
            return Err(AppError(Status::NotFound));
        }

        let mut new_owner: Member = from_value(pre_new_owner.unwrap().get("member")).unwrap();

        // "Move" owner role
        me.roles.retain(|x| x != "00000000-0000-0000-0000-000000000000");
        new_owner.roles.push("00000000-0000-0000-0000-000000000000".to_string());

        // Remove ownership from the source
        database.execute(
            "UPDATE guilds SET members = array_replace(members,
            (
            SELECT member
            FROM unnest(members) AS member
            WHERE member->>'id' = $1
            ),
            $2
            ) WHERE id = $3",
            &[&user_id.0, &to_value(me).unwrap(), &uuid::Uuid::parse_str(&guild_id).unwrap()],
        ).await?;

        // Add ownership to the target
        database.execute(
            "UPDATE guilds SET members = array_replace(members,
            (
            SELECT member
            FROM unnest(members) AS member
            WHERE member->>'id' = $1
            ),
            $2
            ) WHERE id = $3",
            &[&body.owner, &to_value(new_owner).unwrap(), &uuid::Uuid::parse_str(&guild_id).unwrap()],
        ).await?;
    }

    // TODO: Implement other editing

    if body.name.is_some() {
        if body.name.as_ref().unwrap().len() > 30 {
            return Err(AppError(Status::BadRequest));
        }

        database.execute(
            "UPDATE guilds SET name = $1 WHERE id = $2",
            &[&body.name.as_ref().unwrap(), &uuid::Uuid::parse_str(&guild_id).unwrap()],
        ).await?;
    }

    if body.description.is_some() {
        if body.description.as_ref().unwrap().len() > 1000 {
            return Err(AppError(Status::BadRequest));
        }

        database.execute(
            "UPDATE guilds SET description = $1 WHERE id = $2",
            &[&body.description.as_ref().unwrap(), &uuid::Uuid::parse_str(&guild_id).unwrap()],
        ).await?;
    }

    if body.public.is_some() {
        database.execute(
            "UPDATE guilds SET public = $1 WHERE id = $2",
            &[&body.public.as_ref().unwrap(), &uuid::Uuid::parse_str(&guild_id).unwrap()],
        ).await?;
    }

    guild = database
    .query_one(
        "SELECT * FROM guilds WHERE id = $1",
        &[&Uuid::parse_str(guild_id).unwrap()],
    ) 
    .await?;

    let returned_guild = ReturnedGuild {
        id: guild.get::<&str, Uuid>("id").to_string(),
        name: guild.get::<&str, String>("name"),
        description: guild
            .try_get::<&str, Option<String>>("description")
            .unwrap_or(None),
        icon: guild
            .try_get::<&str, Option<String>>("icon")
            .unwrap_or(None),
        public: guild.get::<&str, bool>("public"),
        roles: serde_json::from_value(Value::Array(
            guild.get::<&str, Vec<Value>>("roles"),
        ))
        .unwrap(),
        members: guild
            .get::<&str, Vec<Value>>("members")
            .len(),
        creation: guild.get::<&str, i64>("creation"),
    };

    let members: Vec<Member> = from_value(Value::Array(guild.get::<&str, Vec<Value>>("members"))).unwrap();

    // Broadcast guildEdited event to every member
    for member in members {
        utils::sse::broadcast(
            sse_clients,
            &member.id,
            utils::structs::SSEEvent {
                event: "guildEdited",
                guild: Some(&returned_guild),
                ..Default::default()
            },
        ).await;
    }

    Ok(Json(returned_guild))
    
}

#[delete("/guilds/<guild_id>", format = "json")]
async fn del_guild(
    guild_id: &str,
    sse_clients: &State<crate::SSEClients>,
    database: &State<tokio_postgres::Client>,
    user_id: Auth,
) -> Result<Json<HashMap<String, String>>, AppError> {
    // Get guild
    let pre_guild = database
        .query_one(
            "SELECT * FROM guilds WHERE id = $1 AND EXISTS (
               SELECT 1
               FROM unnest(members) AS member
               WHERE member->>'id' = $2
           )",
            &[&Uuid::parse_str(guild_id).unwrap(), &user_id.0],
        ) 
        .await;

    if pre_guild.is_err() {
        return Err(AppError(Status::NotFound));
    }

    // Get the current user
    let pre_me = database
        .query_one(
            "SELECT member FROM guilds,
            unnest(members) AS member
            WHERE id = $1
            AND member->>'id' = $2",
            &[&Uuid::parse_str(guild_id).unwrap(), &user_id.0],
        ) 
        .await;
    let me: Member = from_value(pre_me.unwrap().get("member")).unwrap();

    // Check if owner
    if !me.roles.contains(&"00000000-0000-0000-0000-000000000000".to_string()) {
        return Err(AppError(Status::Forbidden));
    }

    // Delete guild
    database.execute(
            "DELETE FROM guilds WHERE id = $1",
            &[&uuid::Uuid::parse_str(&guild_id).unwrap()],
        ).await?;

    let members: Vec<Member> = from_value(Value::Array(pre_guild.unwrap().get::<&str, Vec<Value>>("members"))).unwrap();

    // Broadcast guildEdited event to every member
    for member in members {
        utils::sse::broadcast(
            sse_clients,
            &member.id,
            utils::structs::SSEEvent {
                event: "guildLeft",
                guild_id: Some(guild_id),
                ..Default::default()
            },
        ).await;
    }

    Ok(Json(HashMap::new()))
}

// Return routes
pub fn get_routes() -> Vec<Route> {
    routes![get_guild, create_guild, update_guild, del_guild]
}
