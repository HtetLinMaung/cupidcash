use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::models::user::User;
use crate::utils::jwt;
use actix_web::{post, web, HttpResponse};
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
    pub token: Option<String>,
    pub name: Option<String>,
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
                    token: Some(token),
                    name: Some(user.name),
                })
            } else {
                HttpResponse::Unauthorized().json(LoginResponse {
                    code: 401,
                    message: String::from("Invalid password!"),
                    token: None,
                    name: None,
                })
            }
        }
        None => HttpResponse::Unauthorized().json(LoginResponse {
            code: 401,
            message: String::from("Invalid username!"),
            token: None,
            name: None,
        }),
    }
}

async fn get_user(username: &str, client: &web::Data<Arc<Client>>) -> Option<User> {
    // Here we fetch the user from the database using tokio-postgres
    // In a real-world scenario, handle errors gracefully
    let result = client
        .query_one(
            "select u.id, u.username, u.password, r.role_name, u.name, u.shop_id from users u inner join roles r on r.id = u.role_id where username = $1 and u.deleted_at is null and r.deleted_at is null",
            &[&username],
        )
        .await;

    match result {
        Ok(row) => Some(User {
            id: row.get(0),
            username: row.get(1),
            password: row.get(2),
            role_name: row.get(3),
            name: row.get(4),
            shop_id: row.get(5),
        }),
        Err(_) => None,
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
