/*
Copyright (C) 2024  TinyBlueSapling
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

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use rocket::serde::json::Json;
use rocket::{http::Status, Route, State};

use crate::{utils, AppError, Auth};

use super::structs::{PatchMeBody, ReturnedGuild, ReturnedUser, ReturnedUserMe};

#[get("/users/@me")]
async fn get_me(
    database: &State<tokio_postgres::Client>,
    user_id: Auth,
) -> Result<Json<ReturnedUserMe>, AppError> {
    // Get user
    let user = database
        .query_one(
            "SELECT * FROM users WHERE id = $1",
            &[&uuid::Uuid::parse_str(&user_id.0).unwrap()],
        )
        .await?;

    Ok(Json(ReturnedUserMe {
        id: user.get::<&str, uuid::Uuid>("id").to_string(),
        email: user.get::<&str, String>("email"),
        username: user.get::<&str, String>("username"),
        discriminator: user.get::<&str, String>("discriminator"),
        avatar: user
            .try_get::<&str, Option<String>>("avatar")
            .unwrap_or(None),
        about: user
            .try_get::<&str, Option<String>>("about")
            .unwrap_or(None),
        tfa: user.try_get::<&str, String>("otp").is_ok(),
        creation: user.get::<&str, i64>("creation"),
    }))
}

#[delete("/users/@me")]
async fn del_me(
    database: &State<tokio_postgres::Client>,
    user_id: Auth,
) -> Result<Json<()>, AppError> {
    // Delete user
    if database
        .execute(
            "DELETE FROM users WHERE id = $1 AND type = 'USER'",
            &[&uuid::Uuid::parse_str(&user_id.0).unwrap()],
        )
        .await?
        < 1
    {
        return Err(AppError(Status::Forbidden));
    }

    Ok(Json(()))
}

#[patch("/users/@me", format = "json", data = "<body>")]
async fn patch_me(
    body: Json<PatchMeBody>,
    sse_clients: &State<crate::SSEClients>,
    database: &State<tokio_postgres::Client>,
    user_id: Auth,
) -> Result<Json<ReturnedUserMe>, AppError> {
    // Get user
    let user = database
        .query_one(
            "SELECT * FROM users WHERE id = $1",
            &[&uuid::Uuid::parse_str(&user_id.0).unwrap()],
        )
        .await?;

    // Check if current password is correct
    if Argon2::default()
        .verify_password(
            body.current_password.as_bytes(),
            &PasswordHash::new(&user.get::<&str, &str>("password")).unwrap(),
        )
        .is_err()
    {
        return Err(AppError(Status::Unauthorized));
    }

    // Check if username is too long
    if body.username.is_some() && body.username.as_ref().unwrap().len() > 30 {
        return Err(AppError(Status::BadRequest));
    }

    // Check if about is too long
    if body.about.is_some() && body.about.as_ref().unwrap().len() > 1000 {
        return Err(AppError(Status::BadRequest));
    }

    if body.discriminator.is_some() {
        // Check if discriminator is valid
        if !(body.discriminator.as_ref().unwrap().len() == 4
            && body.discriminator.as_ref().unwrap().parse::<u16>().is_ok())
        {
            return Err(AppError(Status::BadRequest));
        }

        // Check if discriminator is unique
        if database
            .query_one(
                "SELECT * FROM users WHERE username = $1 AND discriminator = $2",
                &[
                    &body
                        .username
                        .as_ref()
                        .unwrap_or(&user.get::<&str, String>("username")),
                    &body.discriminator.as_ref().unwrap(),
                ],
            )
            .await
            .is_ok()
        {
            return Err(AppError(Status::Conflict));
        }
    }

    // Generate new token
    let token =
        utils::account::generate_token(user.get::<&str, uuid::Uuid>("id").to_string()).unwrap();

    // Hash new password
    let pseudo_password = Argon2::default()
        .hash_password(
            body.password.as_ref().unwrap_or(&"".to_string()).as_bytes(),
            &SaltString::generate(&mut OsRng),
        )
        .unwrap()
        .to_string();

    // Check if password has been changed
    let new_password = if body.password.as_ref().is_some() {
        &pseudo_password
    } else {
        &user.get::<&str, String>("password")
    };

    // TODO: Allow email changes
    database.execute("UPDATE users SET username = $1, discriminator = $2, about = $3, email = $4, password = $5, token = $6 WHERE id = $7",
    &[
        &body.username.as_ref().unwrap_or(&user.get::<&str, String>("username")), 
        &body.discriminator.as_ref().unwrap_or(&user.get::<&str, String>("discriminator")),
        &body.about.as_ref().unwrap_or(&user.try_get::<&str, String>("about").unwrap_or("".to_string())),
        /*&body.email.as_ref().unwrap_or(*/&user.get::<&str, String>("email")/*)*/,
        &new_password,
        &token,
        &uuid::Uuid::parse_str(&user_id.0).unwrap()
    ]).await?;

    // Get user with new data
    let final_user = database
        .query_one(
            "SELECT * FROM users WHERE id = $1",
            &[&uuid::Uuid::parse_str(&user_id.0).unwrap()],
        )
        .await?;

    let returned_user = ReturnedUserMe {
        id: final_user.get::<&str, uuid::Uuid>("id").to_string(),
        email: final_user.get::<&str, String>("email"),
        username: final_user.get::<&str, String>("username"),
        discriminator: final_user.get::<&str, String>("discriminator"),
        avatar: user
            .try_get::<&str, Option<String>>("avatar")
            .unwrap_or(None),
        about: user
            .try_get::<&str, Option<String>>("about")
            .unwrap_or(None),
        tfa: final_user.try_get::<&str, String>("otp").is_ok(),
        creation: final_user.get::<&str, i64>("creation"),
    };

    // Broadcast userEdited event
    utils::sse::broadcast(sse_clients, &user_id.0, utils::structs::SSEEvent {
        event: "userEdited",
        user: Some(&returned_user),
        ..Default::default()
    }).await;

    Ok(Json(returned_user))
}

#[get("/users/@me/guilds")]
async fn get_my_guilds(
    database: &State<tokio_postgres::Client>,
    user_id: Auth,
) -> Result<Json<Vec<ReturnedGuild>>, AppError> {
    // Get guilds
    let guilds = database
        .query(
            "SELECT * FROM guilds WHERE EXISTS (
                    SELECT 1
                    FROM unnest(members) AS member
                    WHERE member->>'id' = $1
                );",
            &[&user_id.0],
        )
        .await?;

    // Parse guilds
    let mut returned_guilds: Vec<ReturnedGuild> = Vec::new();
    for guild in guilds.iter() {
        returned_guilds.push(ReturnedGuild {
            id: guild.get::<&str, uuid::Uuid>("id").to_string(),
            name: guild.get::<&str, String>("name"),
            description: guild
                .try_get::<&str, Option<String>>("description")
                .unwrap_or(None),
            icon: guild
                .try_get::<&str, Option<String>>("icon")
                .unwrap_or(None),
            public: guild.get::<&str, bool>("public"),
            roles: guild.get::<&str, Vec<rocket::serde::json::Value>>("roles"),
            members: guild
                .get::<&str, Vec<rocket::serde::json::Value>>("members")
                .len(),
            creation: guild.get::<&str, i64>("creation"),
        })
    }

    Ok(Json(returned_guilds))
}

#[get("/users/<user_id>")]
async fn get_user(
    user_id: &str,
    database: &State<tokio_postgres::Client>,
    _user_id: Auth,
) -> Result<Json<ReturnedUser>, AppError> {
    // Get user
    let pre_user = database
        .query_one(
            "SELECT * FROM users WHERE id = $1",
            &[&uuid::Uuid::parse_str(user_id).unwrap()],
        )
        .await;

    if pre_user.is_err() {
        return Err(AppError(Status::NotFound));
    }

    let user = pre_user.unwrap();

    Ok(Json(ReturnedUser {
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
    }))
}

// Return routes
pub fn get_routes() -> Vec<Route> {
    routes![get_me, del_me, patch_me, get_my_guilds, get_user]
}
