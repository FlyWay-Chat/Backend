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

pub mod structs;

pub mod account;
pub mod users;
pub mod experimenting;

// Return routes
pub fn get_routes() -> Vec<rocket::Route> {
    let mut routes = Vec::new();
    routes.extend(crate::utils::sse::get_route());
    
    routes.extend(experimenting::get_routes());
    routes.extend(account::get_routes());
    routes.extend(users::get_routes());

    routes
}
