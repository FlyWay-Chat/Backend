/*
Copyright (C) 2024-2025  BeTalky Community
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

use rocket::{
    http::Status,
    request::{FromRequest, Outcome, Request},
    response::{stream::Event, Responder, Result},
    tokio::sync::{mpsc, Mutex},
};
use std::sync::Arc;
use tokio_postgres::Client;

#[macro_use]
extern crate rocket;
extern crate dotenv;

mod routes;
mod utils;

pub type SSEClients = Arc<Mutex<Vec<(String, mpsc::UnboundedSender<Event>)>>>;

pub struct Auth(String);
#[rocket::async_trait]
impl<'r> FromRequest<'r> for Auth {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> Outcome<Auth, ()> {
        let auth_header = request.headers().get_one("Authorization").unwrap_or("");
        let parts: Vec<&str> = auth_header.split_whitespace().collect();
        let token = parts.last().unwrap_or(&"");

        if !utils::account::validate_token(token) {
            return Outcome::Forward(Status::Unauthorized);
        }

        let database = request.rocket().state::<Client>().unwrap();

        let user = database
            .query_one("SELECT * FROM users WHERE token = $1", &[&token])
            .await;

        if user.is_err() {
            return Outcome::Forward(Status::Unauthorized);
        }

        Outcome::Success(Auth(
            user.unwrap().get::<&str, uuid::Uuid>("id").to_string(),
        ))
    }
}

pub struct AppError(Status);

impl From<tokio_postgres::Error> for AppError {
    fn from(_: tokio_postgres::Error) -> Self {
        AppError(Status::InternalServerError)
    }
}

impl<'r> Responder<'r, 'static> for AppError {
    fn respond_to(self, _: &'r Request<'_>) -> Result<'static> {
        Err(self.0)
    }
}

#[launch]
async fn rocket() -> _ {
    // Initialize
    dotenv::dotenv().ok();
    let database = utils::database::connect().await.unwrap();
    let sse_clients: SSEClients = Arc::new(Mutex::new(vec![]));

    // Routes
    rocket::build()
        .manage(sse_clients)
        .manage(database)
        .mount("/", routes::get_routes())
}
