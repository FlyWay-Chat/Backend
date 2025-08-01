/*
Copyright (C) 2025  FlyWay Chat
This file is part of FlyWay Chat.

FlyWay Chat is free software: you can redistribute it and/or modify
it under the terms of the GNU Affero General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

FlyWay Chat is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU Affero General Public License for more details.

You should have received a copy of the GNU Affero General Public License
along with FlyWay Chat.  If not, see <https://www.gnu.org/licenses/>.
*/

use super::structs::{
    Channel, ChannelRole, CreateGuildBody, Guild, Member, PatchGuildBody, ReturnedGuild, Role,
};
use crate::{
    routes::structs::ReturnedUser,
    to_json_array,
    utils::{
        self,
        permissions::{check_guild_permission, ChannelPermissions, GuildPermissions},
    },
    AppError, Auth,
};

use rocket::{
    http::Status,
    serde::json::{from_value, serde_json, to_value, Json, Value},
    Route, State,
};
use std::{
    collections::HashMap,
    time::{SystemTime, UNIX_EPOCH},
};
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
        roles: serde_json::from_value(Value::Array(guild.get::<&str, Vec<Value>>("roles")))
            .unwrap(),
        members: guild.get::<&str, Vec<Value>>("members").len(),
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
                    permissions: ChannelPermissions::ADMINISTRATOR.bits(),
                },
                ChannelRole {
                    id: "11111111-1111-1111-1111-111111111111".to_string(),
                    permissions: (ChannelPermissions::VIEW_CHANNEL
                        | ChannelPermissions::SEND_MESSAGES)
                        .bits(),
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
                permissions: (GuildPermissions::CREATE_INVITE | GuildPermissions::CHANGE_NICKNAME)
                    .bits(),
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
    // Check if name or description are too long
    if (body.name.is_some() && body.name.as_ref().unwrap().len() > 30)
        || (body.description.is_some() && body.description.as_ref().unwrap().len() > 1000)
    {
        return Err(AppError(Status::BadRequest));
    }

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

    // Get members
    let members: Vec<Member> =
        from_value(Value::Array(guild.get::<&str, Vec<Value>>("members"))).unwrap();
    // Get the current user (as member)
    let mut me: Member = members.iter().find(|x| x.id == user_id.0).unwrap().clone();

    // Check if can manage the guild
    if !check_guild_permission(&guild, &me.id, GuildPermissions::MANAGE_GUILD) {
        return Err(AppError(Status::Forbidden));
    }

    // Transferring ownership
    if body.owner.is_some() {
        // Check if owner
        if !me
            .roles
            .contains(&"00000000-0000-0000-0000-000000000000".to_string())
        {
            return Err(AppError(Status::Forbidden));
        }

        // Check if the new owner exists
        let pre_new_owner = members.iter().find(|x| x.id == body.owner.clone().unwrap());

        if pre_new_owner.is_none() {
            return Err(AppError(Status::NotFound));
        }

        let mut new_owner = pre_new_owner.unwrap().clone();

        // "Move" owner role
        me.roles
            .retain(|role_id| role_id != "00000000-0000-0000-0000-000000000000");
        new_owner
            .roles
            .push("00000000-0000-0000-0000-000000000000".to_string());

        database
            .execute(
                "UPDATE guilds SET members = array_replace(members,
            (
            SELECT member
            FROM unnest(members) AS member
            WHERE member->>'id' = $1
            ),
            $2
            ) WHERE id = $5
             
                UPDATE guilds SET members = array_replace(members,
            (
            SELECT member
            FROM unnest(members) AS member
            WHERE member->>'id' = $3
            ),
            $4
            ) WHERE id = $5",
                &[
                    &user_id.0,
                    &to_value(me).unwrap(),
                    &body.owner,
                    &to_value(new_owner).unwrap(),
                    &uuid::Uuid::parse_str(&guild_id).unwrap(),
                ],
            )
            .await?;
    }

    let final_guild = ReturnedGuild {
        id: guild.get::<&str, Uuid>("id").to_string(),
        name: if body.name.is_some() {
            body.name.as_ref().unwrap().to_string()
        } else {
            guild.get::<&str, String>("name")
        },
        description: if body.description.is_some() {
            Some(body.description.as_ref().unwrap().to_string())
        } else {
            guild
                .try_get::<&str, Option<String>>("description")
                .unwrap_or(None)
        },
        icon: guild
            .try_get::<&str, Option<String>>("icon")
            .unwrap_or(None),
        public: if body.public.is_some() {
            body.public.unwrap()
        } else {
            guild.get::<&str, bool>("public")
        },
        roles: serde_json::from_value(Value::Array(guild.get::<&str, Vec<Value>>("roles")))
            .unwrap(),
        members: guild.get::<&str, Vec<Value>>("members").len(),
        creation: guild.get::<&str, i64>("creation"),
    };

    database
        .execute(
            "UPDATE guilds SET name = $1, description = $2, public = $3 WHERE id = $4",
            &[
                &final_guild.name,
                &final_guild.description,
                &final_guild.public,
                &uuid::Uuid::parse_str(&guild_id).unwrap(),
            ],
        )
        .await?;

    // Broadcast guildEdited event to every member
    for member in members {
        utils::sse::broadcast(
            sse_clients,
            &member.id,
            utils::structs::SSEEvent {
                event: "guildEdited",
                guild: Some(&final_guild),
                ..Default::default()
            },
        )
        .await;
    }

    Ok(Json(final_guild))
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

    // Get the current user (as member)
    let me: Member = from_value(
        pre_guild
            .as_ref()
            .unwrap()
            .get::<&str, Vec<Value>>("members")
            .iter()
            .find(|x| &from_value::<Member>((*x).clone()).unwrap().id == &user_id.0)
            .unwrap()
            .clone(),
    )
    .unwrap();

    // Check if owner
    if !me
        .roles
        .contains(&"00000000-0000-0000-0000-000000000000".to_string())
    {
        return Err(AppError(Status::Forbidden));
    }

    // Delete guild
    database
        .execute(
            "DELETE FROM guilds WHERE id = $1",
            &[&uuid::Uuid::parse_str(&guild_id).unwrap()],
        )
        .await?;

    let members: Vec<Member> = from_value(Value::Array(
        pre_guild.unwrap().get::<&str, Vec<Value>>("members"),
    ))
    .unwrap();

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
        )
        .await;
    }

    Ok(Json(HashMap::new()))
}

#[get("/guilds/<guild_id>/bans", format = "json")]
async fn get_guild_bans(
    guild_id: &str,
    database: &State<tokio_postgres::Client>,
    user_id: Auth,
) -> Result<Json<Vec<ReturnedUser>>, AppError> {
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

    // Check if can manage the guild's bans
    if !check_guild_permission(&guild, &user_id.0, GuildPermissions::BAN_MEMBERS) {
        return Err(AppError(Status::Forbidden));
    }

    let users = database
        .query(
            "SELECT * FROM users WHERE id = any($1)",
            &[
                &from_value::<Vec<Uuid>>(Value::Array(guild.get::<&str, Vec<Value>>("bans")))
                    .unwrap(),
            ],
        )
        .await?;

    Ok(Json(
        users
            .iter()
            .map(|user| ReturnedUser {
                id: user.get::<&str, uuid::Uuid>("id").to_string(),
                username: user.get::<&str, String>("username"),
                discriminator: user.get::<&str, String>("discriminator"),
                avatar: user
                    .try_get::<&str, Option<String>>("avatar")
                    .unwrap_or(None),
                about: user
                    .try_get::<&str, Option<String>>("about")
                    .unwrap_or(None),
                creation: user.get::<&str, i64>("creation"),
            })
            .collect(),
    ))
}

#[delete("/guilds/<guild_id>/bans/<banned_id>", format = "json")]
async fn del_guild_ban(
    guild_id: &str,
    banned_id: &str,
    sse_clients: &State<crate::SSEClients>,
    database: &State<tokio_postgres::Client>,
    user_id: Auth,
) -> Result<Json<ReturnedUser>, AppError> {
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

    // Check if can manage the guild's bans
    if !check_guild_permission(&guild, &user_id.0, GuildPermissions::BAN_MEMBERS) {
        return Err(AppError(Status::Forbidden));
    }

    // Unban user
    database
        .execute(
            "UPDATE guilds SET bans = array_remove(bans, $1) WHERE id = $2",
            &[
                &to_value(banned_id).unwrap(),
                &uuid::Uuid::parse_str(&guild_id).unwrap(),
            ],
        )
        .await?;

    let user = database
        .query_one(
            "SELECT * FROM users WHERE id = $1",
            &[&uuid::Uuid::parse_str(&banned_id).unwrap()],
        )
        .await?;

    let returned_user = ReturnedUser {
        id: user.get::<&str, uuid::Uuid>("id").to_string(),
        username: user.get::<&str, String>("username"),
        discriminator: user.get::<&str, String>("discriminator"),
        avatar: user
            .try_get::<&str, Option<String>>("avatar")
            .unwrap_or(None),
        about: user
            .try_get::<&str, Option<String>>("about")
            .unwrap_or(None),
        creation: user.get::<&str, i64>("creation"),
    };

    let members: Vec<Member> =
        from_value(Value::Array(guild.get::<&str, Vec<Value>>("members"))).unwrap();

    // Broadcast memberUnbanned event to every member that can ban others
    for member in members {
        if check_guild_permission(&guild, &member.id, GuildPermissions::BAN_MEMBERS) {
            utils::sse::broadcast(
                sse_clients,
                &member.id,
                utils::structs::SSEEvent {
                    event: "memberUnbanned",
                    guild_id: Some(guild_id),
                    member: Some(&returned_user),
                    ..Default::default()
                },
            )
            .await;
        }
    }

    Ok(Json(returned_user))
}

// Return routes
pub fn get_routes() -> Vec<Route> {
    routes![
        get_guild,
        create_guild,
        update_guild,
        del_guild,
        get_guild_bans,
        del_guild_ban
    ]
}
