/*
Copyright (C) 2025  BeTalky Community
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

use bitflags::bitflags;
use tokio_postgres::Row;

use crate::routes::structs::{Member, Role};

bitflags! {
    #[derive(Copy, Clone, Debug)]
    pub struct GuildPermissions: usize {
        const CREATE_INVITE = 1 << 0;
        const CHANGE_NICKNAME = 1 << 1;
        const MANAGE_NICKNAMES = 1 << 2;
        const VIEW_AUDIT_LOG = 1 << 3;
        const MANAGE_ROLES = 1 << 4;
        const KICK_MEMBERS = 1 << 5;
        const BAN_MEMBERS = 1 << 6;
        const MANAGE_GUILD = 1 << 7;

        const ADMINISTRATOR = !0;
    }
}

pub fn check_guild_permission(guild: &Row, member_id: &String, permission: GuildPermissions) -> bool {
    // Get proper members and roles
    let members: Vec<Member> = rocket::serde::json::from_value(rocket::serde::json::Value::Array(guild.get::<&str, Vec<rocket::serde::json::Value>>("members"))).unwrap();
    let roles: Vec<Role> = rocket::serde::json::from_value(rocket::serde::json::Value::Array(guild.get::<&str, Vec<rocket::serde::json::Value>>("roles"))).unwrap();

    // Get the member's roles
    let member_roles_ids = members.iter().find(|x| x.id == *member_id).unwrap().roles.clone();
    let mut member_roles = roles.iter().filter(|x| member_roles_ids.contains(&x.id));

    // Check for the permission in every role, and return
    member_roles.any(|x| GuildPermissions::from_bits_truncate(x.permissions).contains(permission))

}
