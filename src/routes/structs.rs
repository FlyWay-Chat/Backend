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

use rocket::serde::{Deserialize, Serialize};

/* account.rs */

/* POST /signin */
/* body */
#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "rocket::serde")]
pub struct SigninBody {
    pub email: String,
    pub password: String,
    pub otp: Option<String>,
}
/* response */
#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "rocket::serde")]
pub struct SigninResp {
    pub token: String,
}

/* POST /signup */
/* body */
#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "rocket::serde")]
pub struct SignupBody {
    pub email: String,
    pub username: String,
    pub password: String,
}

/* POST /reset/request */
/* body */
#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "rocket::serde")]
pub struct ResetRequestBody {
    pub email: String,
}

/* POST /reset/<code> */
/* body */
#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "rocket::serde")]
pub struct ResetBody {
    pub password: String,
}


/* users.rs */

/* GET /users/@me || PATCH /users/@me */
/* response */
#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "rocket::serde")]
pub struct ReturnedUserMe {
    pub id: String,
    pub email: String,
    pub username: String,
    pub discriminator: String,
    pub avatar: Option<String>,
    pub about: Option<String>,
    pub creation: i64,
    pub tfa: bool,
}

/* PATCH /users/@me */
/* body */
#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "rocket::serde")]
pub struct PatchMeBody {
    pub current_password: String,
    pub password: Option<String>,
    pub username: Option<String>,
    pub email: Option<String>,
    pub discriminator: Option<String>,
    pub about: Option<String>,
}

/* GET /users/@me/guilds */
/* response */
#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "rocket::serde")]
pub struct ReturnedGuild {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub icon: Option<String>,
    pub public: bool,
    pub roles: Vec<Role>,
    pub members: usize,
    pub creation: i64,
}

/* GET /users/<user_id> */
/* response */
#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "rocket::serde")]
pub struct ReturnedUser {
    pub id: String,
    pub username: String,
    pub discriminator: String,
    pub avatar: Option<String>,
    pub about: Option<String>,
    pub creation: i64,
}

/* POST /users/@me/otp */
/* response */
#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "rocket::serde")]
pub struct ReturnedOtp {
    pub secret: String,
    pub uri: String,
    pub qr: String,
}

/* POST /users/@me/otp/<code> || DELETE /users/@me/otp */
/* body */
#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "rocket::serde")]
pub struct SetupOTPBody {
    pub password: String,
    pub otp: String,
}


/* guilds.rs */

/* POST /guilds */
/* body */
#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "rocket::serde")]
pub struct CreateGuildBody {
    pub name: String,
    pub description: Option<String>,
}

/* PATCH /guilds/<guild_id> */
/* body */
#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "rocket::serde")]
pub struct PatchGuildBody {
    pub name: Option<String>,
    pub description: Option<String>,
    pub public: Option<bool>,
    pub owner: Option<String>,
}

/* extra */
#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "rocket::serde")]
pub struct Guild {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub icon: Option<String>,
    pub public: bool,
    pub channels: Vec<Channel>,
    pub roles: Vec<Role>,
    pub members: Vec<Member>,
    pub creation: i64,
    pub bans: Vec<String>,
    pub invites: Vec<Invite>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "rocket::serde")]
pub struct Role {
    pub id: String,
    pub name: String,
    pub color: Option<String>,
    pub hoist: bool,
    pub permissions: usize,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "rocket::serde")]
pub struct Member {
    pub id: String,
    pub nickname: Option<String>,
    pub roles: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "rocket::serde")]
pub struct Invite {
    pub code: String,
    pub author: String,
    pub expiration: i64,
    pub max_uses: usize,
    pub uses: usize,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "rocket::serde")]
pub struct Channel {
    pub id: String,
    pub name: String,
    pub topic: Option<String>,
    pub r#type: String,
    pub creation: i64,
    pub roles: Vec<ChannelRole>,
    pub messages: Vec<Message>,
    pub pins: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "rocket::serde")]
pub struct ChannelRole {
    pub id: String,
    pub permissions: usize,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "rocket::serde")]
pub struct Message {
    pub id: String,
    pub author: String,
    pub content: String,
    pub creation: i64,
    pub edited: i64,
    pub r#type: String,
    pub atachment: Option<String>,
    pub atachment_id: Option<String>,
}
