/*
Copyright (C) 2025  TinyBlueSapling
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

use super::structs::ReturnedGuild;
use crate::{utils, AppError, Auth};

use rocket::{http::Status, serde::json::{Json, serde_json, Value}, Route, State};
use std::collections::HashMap;

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
               WHERE (member::jsonb)->>'id' = $2
           )",
            &[&uuid::Uuid::parse_str(guild_id).unwrap(), &user_id.0],
        )
        .await;

    if pre_guild.is_err() {
        println!("{:?}", pre_guild.err());
        return Err(AppError(Status::NotFound));
    }

    let guild = pre_guild.unwrap();

    Ok(Json(ReturnedGuild {
        id: guild.get::<&str, uuid::Uuid>("id").to_string(),
        name: guild.get::<&str, String>("name"),
        description: guild
            .try_get::<&str, Option<String>>("description")
            .unwrap_or(None),
        icon: guild
            .try_get::<&str, Option<String>>("icon")
            .unwrap_or(None),
        public: guild.get::<&str, bool>("public"),
        roles: serde_json::from_value(Value::Array(guild.get::<&str, Vec<rocket::serde::json::Value>>("roles"))).unwrap(),
        members: guild
            .get::<&str, Vec<rocket::serde::json::Value>>("members")
            .len(),
        creation: guild.get::<&str, i64>("creation"),
    }))
}

// Return routes
pub fn get_routes() -> Vec<Route> {
    routes![get_guild]
}
