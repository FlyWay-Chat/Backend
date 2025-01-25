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

use rocket::{
    http::Status,
    response::stream::{Event, EventStream},
    tokio::sync::mpsc,
    Route, State,
};

#[get("/sse?<token>")]
async fn stream(
    token: &str,
    sse_clients: &State<crate::SSEClients>,
    database: &State<tokio_postgres::Client>,
) -> Result<EventStream![], Status> {
    let user = database
        .query_one("SELECT * FROM users WHERE token = $1", &[&token])
        .await;

    if user.is_err() {
        return Err(Status::Unauthorized);
    }

    let (tx, mut rx) = mpsc::unbounded_channel();

    let mut client_lock = sse_clients.lock().await;
    client_lock.push((user.unwrap().get::<&str, uuid::Uuid>("id").to_string(), tx));

    Ok(EventStream! {
        while let Some(event) = rx.recv().await {
            yield event;
        }
    })
}

pub async fn broadcast(
    sse_clients: &State<crate::SSEClients>,
    id: &str,
    message: crate::utils::structs::SSEEvent<'_>,
) {
    let mut client_lock = sse_clients.lock().await;

    client_lock.retain(|client| {
        if id == client.0 {
            client.1.send(Event::json(&message)).is_ok()
        } else {
            true
        }
    });
}

// Return route
pub fn get_route() -> Vec<Route> {
    routes![stream]
}
