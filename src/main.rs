/*
Copyright (C) 2024 TinyBlueSapling
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
along with BeTalky. If not, see <https://www.gnu.org/licenses/>.
*/

use rocket::http::Status;
use rocket::request::{FromRequest, Outcome, Request};
use rocket::response::stream::Event;
use rocket::tokio::sync::{mpsc, Mutex};
use std::sync::Arc;
use tokio_postgres::Client;

#[macro_use]
extern crate rocket;
extern crate dotenv;

mod routes;
mod utils;

pub type WebsocketClients = Arc<Mutex<Vec<(String, mpsc::UnboundedSender<Event>)>>>;

pub struct Auth(String);
#[rocket::async_trait]
impl<'r> FromRequest<'r> for Auth {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> Outcome<Auth, ()> {
        let token = request.headers().get_one("Authorization").unwrap();

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

#[launch]
async fn rocket() -> _ {
    // Initialize
    dotenv::dotenv().ok();
    let database = utils::database::connect().await.unwrap();
    let websockets: WebsocketClients = Arc::new(Mutex::new(vec![]));

    // Routes
    rocket::build()
        .manage(websockets)
        .manage(database)
        .mount("/", routes::get_routes())
}
