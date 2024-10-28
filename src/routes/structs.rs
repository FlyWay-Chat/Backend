/*
Copyright (C) 2024  TinyBlueSapling
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

/* /signin */
/* body */
#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct SigninBody<'r> {
    pub email: &'r str,
    pub password: &'r str,
    pub otp: Option<&'r str>,
}
/* response */
#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub struct SigninResp {
    pub token: String,
}

/* /signup */
/* body */
#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct SignupBody<'r> {
    pub email: &'r str,
    pub username: &'r str,
    pub password: &'r str,
}

/* /reset/request */
/* body */
#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct ResetRequestBody<'r> {
    pub email: &'r str,
}

/* /reset/<code> */
/* body */
#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct ResetBody<'r> {
    pub password: &'r str,
}
