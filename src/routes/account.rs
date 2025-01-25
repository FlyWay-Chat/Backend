/*
Copyright (C) 2024-2025  TinyBlueSapling
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

use super::structs::{ResetBody, ResetRequestBody, SigninBody, SigninResp, SignupBody};
use crate::{utils, AppError};

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use rocket::{http::Status, serde::json::Json, Route, State};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

#[post("/signin", format = "json", data = "<body>")]
async fn signin(
    body: Json<SigninBody>,
    database: &State<tokio_postgres::Client>,
) -> Result<Json<SigninResp>, AppError> {
    // Check if user exists
    let pre_user = database
        .query_one("SELECT * FROM users WHERE email = $1", &[&body.email])
        .await;

    if pre_user.is_err() {
        return Err(AppError(Status::Unauthorized));
    }

    let user = pre_user.unwrap();

    // Check if password is correct
    if Argon2::default()
        .verify_password(
            body.password.as_bytes(),
            &PasswordHash::new(&user.get::<&str, String>("password")).unwrap(),
        )
        .is_err()
    {
        return Err(AppError(Status::Unauthorized));
    }

    // Check if user is verified
    if !user.get::<&str, bool>("verified") {
        return Err(AppError(Status::PreconditionRequired));
    }

    // Verify OTP
    let tfa_secret = &user.try_get::<&str, String>("otp");
    if !tfa_secret.is_err()
        && !utils::account::verify_otp(
            tfa_secret.as_ref().unwrap(),
            &body.otp.clone().unwrap_or(String::new()),
        )
    {
        return Err(AppError(Status::Unauthorized));
    }

    let mut token = user
        .try_get::<&str, String>("token")
        .unwrap_or(String::new());

    // Generate token if not exists
    if !utils::account::validate_token(token.as_str()) {
        token = utils::account::generate_token(user.get::<&str, Uuid>("id").to_string()).unwrap();
        database
            .execute(
                "UPDATE users SET token = $1 WHERE email = $2",
                &[&token, &body.email],
            )
            .await?;
    }

    Ok(Json(SigninResp { token }))
}

#[post("/signup", format = "json", data = "<body>")]
async fn signup(
    body: Json<SignupBody>,
    database: &State<tokio_postgres::Client>,
) -> Result<(), AppError> {
    // Check if username is too long
    if body.username.len() > 30 {
        return Err(AppError(Status::BadRequest));
    }

    // Check if user with email exists
    let bad_user = database
        .query_one("SELECT * FROM users WHERE email = $1", &[&body.email])
        .await;

    if !bad_user.is_err() {
        // Check if user with email is verified
        if bad_user.unwrap().get::<&str, bool>("verified") {
            return Err(AppError(Status::Unauthorized));
        } else {
            // Delete unverified user
            database
                .execute("DELETE FROM users WHERE email = $1", &[&body.email])
                .await?;
        }
    }

    // Get all users with same username
    let same_usernames = database
        .query("SELECT * FROM users WHERE username = $1", &[&body.username])
        .await?;

    // Generate discriminator
    let discriminator = utils::account::generate_discriminator(
        &same_usernames
            .iter()
            .map(|row| row.get::<&str, String>("discriminator"))
            .collect::<Vec<String>>(),
    );

    // All discriminators are taken
    if discriminator.is_none() {
        return Err(AppError(Status::Conflict));
    }

    // Create user
    let id = Uuid::new_v4();
    let password = Argon2::default()
        .hash_password(body.password.as_bytes(), &SaltString::generate(&mut OsRng))
        .unwrap()
        .to_string();
    let token = utils::account::generate_token(id.to_string()).unwrap();
    let verificator = Uuid::new_v4().to_string();

    database.execute("INSERT INTO users (id, token, email, password, username, discriminator, avatar, creation, type, verified, verificator) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)", &[&id, &token, &body.email, &password, &body.username, &discriminator, &"userDefault", &(SystemTime::now()
    .duration_since(UNIX_EPOCH).unwrap().as_secs() as i64), &"USER", &false, &verificator]).await?;

    Ok(())
}

#[post("/verify/<code>", format = "json")]
async fn verify(
    code: &str,
    database: &State<tokio_postgres::Client>,
) -> Result<Json<SigninResp>, AppError> {
    // Check if user exists
    let pre_user = database
        .query_one("SELECT * FROM users WHERE verificator = $1", &[&code])
        .await;

    if pre_user.is_err() {
        return Err(AppError(Status::Unauthorized));
    }

    // Verify user
    database
        .execute(
            "UPDATE users SET verified = $1, verificator = $2 WHERE verificator = $3",
            &[&true, &"", &code],
        )
        .await?;

    Ok(Json(SigninResp {
        token: pre_user
            .unwrap()
            .try_get::<&str, String>("token")
            .unwrap_or(String::new()),
    }))
}

#[post("/reset/request", format = "json", data = "<body>")]
async fn reset_request(
    body: Json<ResetRequestBody>,
    database: &State<tokio_postgres::Client>,
) -> Result<(), AppError> {
    // Check if user exists
    let pre_user = database
        .query_one("SELECT * FROM users WHERE email = $1", &[&body.email])
        .await;

    if pre_user.is_err() {
        return Err(AppError(Status::Unauthorized));
    }

    let verificator = Uuid::new_v4().to_string();

    // TODO: email the verificator

    database
        .execute(
            "UPDATE users SET verificator = $1 WHERE email = $2",
            &[&verificator, &body.email],
        )
        .await?;

    Ok(())
}

#[get("/reset/<code>", format = "json")]
async fn reset_check(code: &str, database: &State<tokio_postgres::Client>) -> Result<(), AppError> {
    // Check if user exists
    let pre_user = database
        .query_one("SELECT * FROM users WHERE verificator = $1", &[&code])
        .await;

    if pre_user.is_err() {
        return Err(AppError(Status::Unauthorized));
    }

    // TODO: Implement reset check

    Ok(())
}

#[post("/reset/<code>", format = "json", data = "<body>")]
async fn reset(
    body: Json<ResetBody>,
    code: &str,
    database: &State<tokio_postgres::Client>,
) -> Result<Json<SigninResp>, AppError> {
    // Check if user exists
    let pre_user = database
        .query_one("SELECT * FROM users WHERE verificator = $1", &[&code])
        .await;

    if pre_user.is_err() {
        return Err(AppError(Status::Unauthorized));
    }

    // Generate token
    let token =
        utils::account::generate_token(pre_user.unwrap().get::<&str, Uuid>("id").to_string())
            .unwrap();

    // Hash password
    let password = Argon2::default()
        .hash_password(body.password.as_bytes(), &SaltString::generate(&mut OsRng))
        .unwrap()
        .to_string();

    database
        .execute(
            "UPDATE users SET token = $1, password = $2, verificator = $3 WHERE verificator = $4",
            &[&token, &password, &"", &code],
        )
        .await?;

    Ok(Json(SigninResp { token }))
}

// Return routes
pub fn get_routes() -> Vec<Route> {
    routes![signin, signup, verify, reset_request, reset_check, reset]
}
