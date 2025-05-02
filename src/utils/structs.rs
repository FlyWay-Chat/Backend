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

use crate::routes::structs::{ReturnedGuild, ReturnedUser, ReturnedUserMe};

use rocket::serde::Serialize;

#[macro_export]
macro_rules! to_json_array {
    ($x:expr) => {
        &$x.iter()
            .map(|x| serde_json::to_value(x).unwrap())
            .collect::<Vec<serde_json::Value>>()
    };
}

#[derive(Serialize, Debug)]
#[serde(crate = "rocket::serde")]
pub struct SSEEvent<'r> {
    pub event: &'r str,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<&'r ReturnedUserMe>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub guild: Option<&'r ReturnedGuild>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub guild_id: Option<&'r str>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<&'r str>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub member: Option<&'r ReturnedUser>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel: Option<&'r str>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<&'r str>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub invite: Option<&'r str>,
}

impl Default for SSEEvent<'_> {
    fn default() -> SSEEvent<'static> {
        SSEEvent {
            event: "unknown",
            user: None,
            guild: None,
            guild_id: None,
            role: None,
            member: None,
            channel: None,
            message: None,
            invite: None,
        }
    }
}
