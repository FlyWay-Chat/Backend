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

use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use rocket::serde::{Deserialize, Serialize};
use std::{
    env,
    time::{SystemTime, UNIX_EPOCH},
};
use totp_rs::{Rfc6238, Secret, TOTP};

#[derive(Debug, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
struct Claims {
    sub: String,
    iss: String,
    exp: u64,
}

pub fn generate_token(id: String) -> Result<String, jsonwebtoken::errors::Error> {
    let claims = Claims {
        sub: id.to_owned(),
        iss: "betalky".to_owned(),
        exp: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            + 604_800, /* 7d */
    };
    let jwt_key = env::var("JWT_KEY").unwrap();
    let key = jwt_key.as_bytes();

    let header = Header {
        alg: Algorithm::HS512,
        ..Default::default()
    };

    let token = encode(&header, &claims, &EncodingKey::from_secret(key));

    if token.is_err() {
        return Err(token.unwrap_err());
    }

    Ok(token.unwrap())
}

pub fn validate_token(token: &str) -> bool {
    let jwt_key = env::var("JWT_KEY").unwrap();
    let key = jwt_key.as_bytes();

    return decode::<Claims>(
        &token,
        &DecodingKey::from_secret(key),
        &Validation::new(Algorithm::HS512),
    )
    .is_ok();
}

pub fn verify_otp(secret: &str, code_to_check: &str) -> bool {
    let rfc =
        Rfc6238::with_defaults(Secret::Encoded(secret.to_string()).to_bytes().unwrap()).unwrap();

    let totp = TOTP::from_rfc6238(rfc).unwrap();
    let code = totp.generate_current().unwrap();

    return code == code_to_check;
}

// TODO: Generate random discriminators
pub fn generate_discriminator(excluded: &[String]) -> Option<String> {
    if excluded.len() >= 10_000 {
        return None;
    }

    for i in 0..10_000 {
        let padded = format!("{:0>4}", i);

        if !excluded.contains(&padded) {
            return Some(padded);
        }
    }

    None
}
