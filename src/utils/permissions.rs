/*
Copyright (C) 2025  FlyWay Chat
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

use bitflags::bitflags;
use rocket::serde::json::{from_value, Value};
use tokio_postgres::Row;

use crate::routes::structs::{Channel, Member, Role};

bitflags! {
    #[derive(Copy, Clone, Debug)]
    pub struct GuildPermissions: i64 {
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

bitflags! {
    #[derive(Copy, Clone, Debug)]
    pub struct ChannelPermissions: i64 {
        const VIEW_CHANNEL = 1 << 0;
        const SEND_MESSAGES = 1 << 1;
        const MANAGE_CHANNEL = 1 << 2;
        const MANAGE_MESSAGES = 1 << 3;

        const ADMINISTRATOR = !0;
    }
}

pub fn check_guild_permission(
    guild: &Row,
    member_id: &String,
    permission: GuildPermissions,
) -> bool {
    // Get proper members and roles
    let members: Vec<Member> =
        from_value(Value::Array(guild.get::<&str, Vec<Value>>("members"))).unwrap();
    let roles: Vec<Role> =
        from_value(Value::Array(guild.get::<&str, Vec<Value>>("roles"))).unwrap();

    // Get the member's roles
    let member_roles_ids = members
        .iter()
        .find(|member| member.id == *member_id)
        .unwrap()
        .roles
        .clone();
    let mut member_roles = roles
        .iter()
        .filter(|role| member_roles_ids.contains(&role.id));

    // Check for the permission in every role, and return
    member_roles
        .any(|role| GuildPermissions::from_bits_truncate(role.permissions).contains(permission))
}

pub fn check_channel_permission(
    guild: &Row,
    channel_id: &String,
    member_id: &String,
    permission: ChannelPermissions,
) -> bool {
    // Get proper members and channel
    let members: Vec<Member> =
        from_value(Value::Array(guild.get::<&str, Vec<Value>>("members"))).unwrap();
    let channels: Vec<Channel> = from_value(Value::Array(guild.get::<&str, Vec<Value>>("channels"))).unwrap();
    let channel = channels.iter().find(|channel| channel.id == *channel_id).unwrap();

    // Get the member's roles
    let member_roles_ids = members
        .iter()
        .find(|member| member.id == *member_id)
        .unwrap()
        .roles
        .clone();
    let mut member_channel_roles = channel.roles
        .iter()
        .filter(|role| member_roles_ids.contains(&role.id));

    // Check for the permission in every role, and return
    member_channel_roles
        .any(|role| ChannelPermissions::from_bits_truncate(role.permissions).contains(permission))
}
