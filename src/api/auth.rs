use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::models::user::get_user;
use crate::utils::common_struct::{BaseResponse, DataResponse};
use crate::utils::jwt::{self, verify_token_and_get_sub};
use actix_web::{post, web, HttpResponse, Responder};
use bcrypt::{hash, verify, DEFAULT_COST};
use serde::{Deserialize, Serialize};
use tokio_postgres::Client;

#[derive(Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    pub code: u16,
    pub message: String,
    pub token: String,
    pub name: String,
    pub role: String,
}

#[post("/api/auth/login")]
pub async fn login(
    client: web::Data<Arc<Client>>,
    credentials: web::Json<LoginRequest>,
) -> HttpResponse {
    // Fetch user from the database based on the username
    let user = get_user(&credentials.username, &client).await;

    match user {
        Some(user) => {
            if verify(&credentials.password, &user.password).unwrap() {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .expect("Time went backwards")
                    .as_secs() as usize;
                let token = jwt::sign_token(&jwt::Claims {
                    sub: format!("{},{},{}", &user.id, &user.role_name, &user.shop_id),
                    exp: now + (3600 * 24),
                })
                .unwrap();
                // let token = create_token(&user.username).unwrap();
                HttpResponse::Ok().json(LoginResponse {
                    code: 200,
                    message: String::from("Token generated successfully."),
                    token: token,
                    name: user.name,
                    role: user.role_name,
                })
            } else {
                HttpResponse::Unauthorized().json(BaseResponse {
                    code: 401,
                    message: String::from("Invalid password!"),
                })
            }
        }
        None => HttpResponse::Unauthorized().json(BaseResponse {
            code: 401,
            message: String::from("Invalid username!"),
        }),
    }
}

#[derive(Deserialize)]
pub struct PasswordInput {
    pub password: String,
}

#[derive(Serialize)]
pub struct HashedPasswordOutput {
    pub hashed_password: String,
}

#[post("/api/hash_password")]
pub async fn hash_password(password_input: web::Json<PasswordInput>) -> HttpResponse {
    match hash(&password_input.password, DEFAULT_COST) {
        Ok(hashed) => HttpResponse::Ok().json(HashedPasswordOutput {
            hashed_password: hashed,
        }),
        Err(_) => HttpResponse::InternalServerError().body("Failed to hash password"),
    }
}

#[derive(Deserialize)]
pub struct VerifyTokenRequest {
    pub token: String,
}

#[derive(Serialize)]
pub struct VerifyTokenData {
    pub room: i32,
}

#[post("/api/auth/verify-token")]
pub async fn verify_token(body: web::Json<VerifyTokenRequest>) -> impl Responder {
    let sub = match verify_token_and_get_sub(&body.token) {
        Some(s) => s,
        None => {
            return HttpResponse::Unauthorized().json(BaseResponse {
                code: 401,
                message: String::from("Invalid token"),
            })
        }
    };

    // Parse the `sub` value
    let parsed_values: Vec<&str> = sub.split(',').collect();
    if parsed_values.len() != 3 {
        return HttpResponse::InternalServerError().json(BaseResponse {
            code: 500,
            message: String::from("Invalid sub format in token"),
        });
    }

    let user_id: i32 = parsed_values[0].parse().unwrap();
    return HttpResponse::Ok().json(DataResponse {
        code: 200,
        message: String::from("Token is valid."),
        data: Some(VerifyTokenData { room: user_id }),
    });
}
