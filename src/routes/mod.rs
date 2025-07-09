/*
Copyright (C) 2024-2025  FlyWay Chat
This file is part of FlyWay Chat.

FlyWay Chat is free software: you can redistribute it and/or modify
it under the terms of the GNU Affero General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

FlyWay Chat is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU Affero General Public License for more details.

You should have received a copy of the GNU Affero General Public License
along with FlyWay Chat.  If not, see <https://www.gnu.org/licenses/>.
*/

use crate::utils;

pub mod structs;

pub mod account;
pub mod experimenting;
pub mod guilds;
pub mod invites;
pub mod users;

// Return routes
pub fn get_routes() -> Vec<rocket::Route> {
    let mut routes = Vec::new();
    routes.extend(utils::sse::get_route());

    routes.extend(experimenting::get_routes());
    routes.extend(account::get_routes());
    routes.extend(users::get_routes());
    routes.extend(guilds::get_routes());
    routes.extend(invites::get_routes());

    routes
}
