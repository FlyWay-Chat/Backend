/*
Copyright (C) 2024 TinyBlueSapling
This file is part of BeTalky.

BeTalky is free software: you can redistribute it and/or modify
it under the terms of the GNU Affero General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

BeTalky is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
GNU Affero General Public License for more details.

You should have received a copy of the GNU Affero General Public License
along with BeTalky. If not, see <https://www.gnu.org/licenses/>.
*/

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use rocket::serde::json::Json;
use rocket::{http::Status, Route, State};

use crate::utils;

use crate::routes::structs::{SigninBody, SigninResp, SignupBody, ResetRequestBody, ResetBody};

#[post("/signin", format = "json", data = "<body>")]
async fn signin(
    body: Json<SigninBody<'_>>,
    database: &State<tokio_postgres::Client>,
) -> Result<Json<SigninResp>, Status> {
    let pre_user = database
        .query_one("SELECT * FROM users WHERE email = $1", &[&body.email])
        .await;

    if pre_user.is_err() {
        return Err(Status::Unauthorized);
    }

    let user = pre_user.unwrap();

    if Argon2::default()
        .verify_password(
            body.password.as_bytes(),
            &PasswordHash::new(&user.get::<&str, &str>("password")).unwrap(),
        )
        .is_err()
    {
        return Err(Status::Unauthorized);
    }

    if !user.get::<&str, bool>("verified") {
        return Err(Status::PreconditionRequired);
    }

    let tfa_secret = &user.try_get::<&str, &str>("otp");
    if !tfa_secret.is_err() && !utils::account::verify_otp(tfa_secret.as_ref().unwrap(), body.otp.unwrap_or("")) {
        return Err(Status::Unauthorized);
    }

    let mut token = user.get::<&str, String>("token");

    if !utils::account::validate_token(token.as_str()) {
        token = utils::account::generate_token(user.get::<&str, uuid::Uuid>("id").to_string()).unwrap();
        if database
            .execute(
                "UPDATE users SET token = $1 WHERE email = $2",
                &[&token, &body.email],
            )
            .await
            .is_err()
        {
            return Err(Status::InternalServerError);
        }
    }

    Ok(Json(SigninResp { token }))
}

#[post("/signup", format = "json", data = "<body>")]
async fn signup(
    body: Json<SignupBody<'_>>,
    database: &State<tokio_postgres::Client>,
) -> Result<(), Status> {
    if body.username.len() > 30 {
        return Err(Status::UnprocessableEntity);
    }

    let bad_user = database
        .query_one("SELECT * FROM users WHERE email = $1", &[&body.email])
        .await;

    if !bad_user.is_err() {
        if bad_user.unwrap().get::<&str, bool>("verified") {
            return Err(Status::Unauthorized);
        } else if database
            .execute("DELETE FROM users WHERE email = $1", &[&body.email])
            .await
            .is_err()
        {
            return Err(Status::InternalServerError);
        }
    }

    let same_usernames = database
        .query("SELECT * FROM users WHERE username = $1", &[&body.username])
        .await;

    if same_usernames.is_err() {
        return Err(Status::InternalServerError);
    }

    // Generate discriminator
    let discriminator = utils::account::generate_discriminator(
        &same_usernames
            .unwrap()
            .iter()
            .map(|row| row.get::<&str, &str>("discriminator"))
            .collect::<Vec<&str>>(),
    );

    if discriminator.is_none() {
        return Err(Status::Conflict);
    }

    // Create user
    let id = uuid::Uuid::new_v4();
    let password = Argon2::default()
        .hash_password(body.password.as_bytes(), &SaltString::generate(&mut OsRng))
        .unwrap()
        .to_string();
    let token = utils::account::generate_token(id.to_string()).unwrap();
    let verificator = uuid::Uuid::new_v4().to_string();

    if database.execute("INSERT INTO users (id, token, email, password, username, discriminator, avatar, creation, type, verified, verificator) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)", &[&id, &token, &body.email, &password, &body.username, &discriminator, &"userDefault", &(std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64), &"USER", &false, &verificator]).await.is_err() {
        return Err(Status::InternalServerError);
    }

    Ok(())
}

#[post("/verify/<code>")]
async fn verify(
    code: &str,
    database: &State<tokio_postgres::Client>,
) -> Result<Json<SigninResp>, Status> {
    let pre_user = database
        .query_one("SELECT * FROM users WHERE verificator = $1", &[&code])
        .await;

    if pre_user.is_err() {
        return Err(Status::Unauthorized);
    }

    if database
        .execute(
            "UPDATE users SET verified = $1, verificator = $2 WHERE verificator = $3",
            &[&true, &"", &code],
        )
        .await
        .is_err()
    {
        return Err(Status::InternalServerError);
    }

    Ok(Json(SigninResp {
        token: pre_user.unwrap().get::<&str, String>("token"),
    }))
}

#[post("/reset/request", format = "json", data = "<body>")]
async fn reset_request(
    body: Json<ResetRequestBody<'_>>,
    database: &State<tokio_postgres::Client>,
) -> Result<(), Status> {
    let pre_user = database
        .query_one("SELECT * FROM users WHERE email = $1", &[&body.email])
        .await;

    if pre_user.is_err() {
        return Err(Status::Unauthorized);
    }

    let verificator = uuid::Uuid::new_v4().to_string();

    // TODO: email the verificator

    if database
        .execute(
            "UPDATE users SET verificator = $1 WHERE email = $2",
            &[&verificator, &body.email],
        )
        .await
        .is_err()
    {
        return Err(Status::InternalServerError);
    }

    Ok(())
}

#[get("/reset/<code>")]
async fn reset_check(
    code: &str,
    database: &State<tokio_postgres::Client>,
) -> Result<(), Status> {
    let pre_user = database
        .query_one("SELECT * FROM users WHERE verificator = $1", &[&code])
        .await;

    if pre_user.is_err() {
        return Err(Status::Unauthorized);
    }

    Ok(())
}

#[post("/reset/<code>", format = "json", data = "<body>")]
async fn reset(
    body: Json<ResetBody<'_>>,
    code: &str,
    database: &State<tokio_postgres::Client>,
) -> Result<Json<SigninResp>, Status> {
    let pre_user = database
        .query_one("SELECT * FROM users WHERE verificator = $1", &[&code])
        .await;

    if pre_user.is_err() {
        return Err(Status::Unauthorized);
    }

    let token =
        utils::account::generate_token(pre_user.unwrap().get::<&str, uuid::Uuid>("id").to_string()).unwrap();
    let password = Argon2::default()
        .hash_password(body.password.as_bytes(), &SaltString::generate(&mut OsRng))
        .unwrap()
        .to_string();

    if database
        .execute(
            "UPDATE users SET token = $1, password = $2, verificator = $3 WHERE verificator = $4",
            &[&token, &password, &"", &code],
        )
        .await
        .is_err()
    {
        return Err(Status::InternalServerError);
    }

    Ok(Json(SigninResp {
        token,
    }))
}

// Return routes
pub fn get_routes() -> Vec<Route> {
    routes![signin, signup, verify, reset_request, reset_check, reset]
}
