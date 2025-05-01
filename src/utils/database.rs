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

use rocket::tokio;
use std::env;
use tokio_postgres::{Client, Error, NoTls};

pub async fn connect() -> Result<Client, Error> {
    // Connect to the database.
    let (client, connection) = tokio_postgres::connect(
        &format!(
            "host={} port={} dbname={} user={}",
            env::var("DB_HOST").unwrap(),
            env::var("DB_PORT").unwrap(),
            env::var("DB_NAME").unwrap(),
            env::var("DB_USER").unwrap()
        ),
        NoTls,
    )
    .await?;

    // The connection object performs the actual communication with the database,
    // so spawn it off to run on its own.
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });

    init(&client).await.unwrap();

    Ok(client)
}

async fn init(database: &Client) -> Result<(), Error> {
    // Initialize database

    database
        .query_opt(
            "CREATE TABLE IF NOT EXISTS users (
        id uuid NOT NULL,
        token text,
        email text NOT NULL,
        password text NOT NULL,
        username text NOT NULL,
        discriminator text NOT NULL,
        avatar text,
        about text,
        creation bigint NOT NULL,
        type text NOT NULL,
        owner text,
        verified boolean NOT NULL,
        verificator text,
        otp text,
        PRIMARY KEY (id)
    )",
            &[],
        )
        .await?;

    database
        .query_opt(
            "CREATE TABLE IF NOT EXISTS guilds (
        id uuid NOT NULL,
        name text NOT NULL,
        description TEXT,
        icon text,
        public boolean NOT NULL,
        channels jsonb[],
        roles jsonb[],
        members jsonb[],
        creation bigint NOT NULL,
        bans jsonb[],
        invites jsonb[],
        PRIMARY KEY (id)
    )",
            &[],
        )
        .await?;

    database
        .query_opt(
            "CREATE TABLE IF NOT EXISTS meta (
        url text,
        creation bigint,
        title text,
        description text,
        image text,
        PRIMARY KEY (url)
    )",
            &[],
        )
        .await?;

    Ok(())
}
