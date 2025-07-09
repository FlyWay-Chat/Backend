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

use super::structs::{CreateInviteBody, Member, ReturnedGuild};
use crate::{
    routes::structs::Invite,
    utils::permissions::{check_guild_permission, GuildPermissions},
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

#[get("/guilds/<guild_id>/invites", format = "json")]
async fn get_guild_invites(
    guild_id: &str,
    database: &State<tokio_postgres::Client>,
    user_id: Auth,
) -> Result<Json<Vec<Invite>>, AppError> {
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

    // Check if can manage the guild
    if !check_guild_permission(&guild, &user_id.0, GuildPermissions::MANAGE_GUILD) {
        return Err(AppError(Status::Forbidden));
    }

    let invites: Vec<Invite> =
        from_value(Value::Array(guild.get::<&str, Vec<Value>>("invites"))).unwrap();

    Ok(Json(invites))
}

#[post("/guilds/<guild_id>/invites", format = "json", data = "<body>")]
async fn create_guild_invite(
    body: Json<CreateInviteBody>,
    guild_id: &str,
    database: &State<tokio_postgres::Client>,
    user_id: Auth,
) -> Result<Json<Invite>, AppError> {
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

    // Check if can create invites
    if !check_guild_permission(&guild, &user_id.0, GuildPermissions::CREATE_INVITE) {
        return Err(AppError(Status::Forbidden));
    }

    // Create invite
    let invite = Invite {
        code: Uuid::new_v4().to_string(),
        author: user_id.0,
        expiration: (SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64)
            + body.expiration,
        max_uses: body.max_uses,
        uses: 0,
    };

    // Append invite
    database
        .execute(
            "UPDATE guilds SET invites = array_append(invites, $1) WHERE id = $2",
            &[
                &to_value(invite.clone()).unwrap(),
                &uuid::Uuid::parse_str(&guild_id).unwrap(),
            ],
        )
        .await?;

    Ok(Json(invite))
}

#[delete("/guilds/<guild_id>/invites/<invite_code>", format = "json")]
async fn delete_guild_invite(
    guild_id: &str,
    invite_code: &str,
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

    let guild = pre_guild.unwrap();

    // Check if can manage the guild
    if !check_guild_permission(&guild, &user_id.0, GuildPermissions::MANAGE_GUILD) {
        return Err(AppError(Status::Forbidden));
    }

    // Delete the invite
    database
        .execute(
            "UPDATE guilds SET invites = array_remove(invites, (
            SELECT invite
            FROM unnest(invites) AS invite
            WHERE invite->>'code' = $1
            )) WHERE id = $2",
            &[&invite_code, &uuid::Uuid::parse_str(&guild_id).unwrap()],
        )
        .await?;

    Ok(Json(HashMap::new()))
}

#[get("/invites/<invite_code>", format = "json")]
async fn get_invite(
    invite_code: &str,
    database: &State<tokio_postgres::Client>,
    user_id: Auth,
) -> Result<Json<ReturnedGuild>, AppError> {
    // Get guild
    let pre_guild = database
        .query_one(
            "SELECT * FROM guilds WHERE EXISTS (
               SELECT 1
               FROM unnest(invites) AS invite
               WHERE invite->>'code' = $1
               AND (invite->'max_uses')::BIGINT > (invite->'uses')::BIGINT
               AND (invite->'expiration')::BIGINT > $2
           ) AND NOT EXISTS (
               SELECT 1
               FROM unnest(bans) AS id
               WHERE id = $3
           )",
            &[
                &invite_code,
                &(SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as i64),
                &to_value(&user_id.0).unwrap(),
            ],
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

#[put("/invites/<invite_code>", format = "json")]
async fn join_invite(
    invite_code: &str,
    database: &State<tokio_postgres::Client>,
    user_id: Auth,
) -> Result<Json<ReturnedGuild>, AppError> {
    // Get guild
    let pre_guild = database
        .query_one(
            "SELECT * FROM guilds WHERE EXISTS (
               SELECT 1
               FROM unnest(invites) AS invite
               WHERE invite->>'code' = $1
               AND (invite->'max_uses')::BIGINT > (invite->'uses')::BIGINT
               AND (invite->'expiration')::BIGINT > $2
           ) AND NOT EXISTS (
               SELECT 1
               FROM unnest(bans) AS id
               WHERE id = $3
           ) AND NOT EXISTS (
               SELECT 1
               FROM unnest(members) AS member
               WHERE member->>'id' = $4
           )",
            &[
                &invite_code,
                &(SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as i64),
                &to_value(&user_id.0).unwrap(),
                &user_id.0,
            ],
        )
        .await;

    if pre_guild.is_err() {
        return Err(AppError(Status::NotFound));
    }

    let guild = pre_guild.unwrap();

    // Get invite
    let invite = database
        .query_one(
            "SELECT invite FROM guilds,
            unnest(invites) AS invite
            WHERE id = $1
            AND invite->>'code' = $2",
            &[&guild.get::<&str, Uuid>("id"), &invite_code],
        )
        .await?;

    let mut new_invite: Invite = from_value(invite.get("invite")).unwrap();

    new_invite.uses += 1;

    // Append member
    database
        .execute(
            "UPDATE guilds SET members = array_append(members, $1),
                                invites = array_replace(invites,
                                    (
                                        SELECT invite
                                        FROM unnest(invites) AS invite
                                        WHERE invite->>'code' = $2
                                    ),
                                    $3
                                )
            WHERE id = $4",
            &[
                &to_value(Member {
                    id: user_id.0,
                    nickname: None,
                    roles: vec!["11111111-1111-1111-1111-111111111111".to_string()],
                })
                .unwrap(),
                &invite_code,
                &to_value(new_invite).unwrap(),
                &guild.get::<&str, Uuid>("id"),
            ],
        )
        .await?;

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

// Return routes
pub fn get_routes() -> Vec<Route> {
    routes![
        get_guild_invites,
        create_guild_invite,
        delete_guild_invite,
        get_invite,
        join_invite
    ]
}
