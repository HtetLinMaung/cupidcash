use std::{sync::Arc, vec};

use crate::{
    models::category::{self, Category},
    utils::jwt::verify_token_and_get_sub,
};
use actix_web::{get, web, HttpRequest, HttpResponse, Responder};
use serde::Serialize;
use tokio_postgres::Client;

#[derive(Serialize)]
pub struct GetCategoriesResponse {
    pub code: u16,
    pub message: String,
    pub data: Vec<Category>,
}

#[get("/api/categories")]
pub async fn get_categories(req: HttpRequest, client: web::Data<Arc<Client>>) -> impl Responder {
    // Extract the token from the Authorization header
    let token = match req.headers().get("Authorization") {
        Some(value) => {
            let parts: Vec<&str> = value.to_str().unwrap_or("").split_whitespace().collect();
            if parts.len() == 2 && parts[0] == "Bearer" {
                parts[1]
            } else {
                return HttpResponse::BadRequest().json(GetCategoriesResponse {
                    code: 400,
                    message: String::from("Invalid Authorization header format"),
                    data: vec![],
                });
            }
        }
        None => {
            return HttpResponse::Unauthorized().json(GetCategoriesResponse {
                code: 401,
                message: String::from("Authorization header missing"),
                data: vec![],
            })
        }
    };

    let sub = match verify_token_and_get_sub(token) {
        Some(s) => s,
        None => {
            return HttpResponse::Unauthorized().json(GetCategoriesResponse {
                code: 401,
                message: String::from("Invalid token"),
                data: vec![],
            })
        }
    };

    // Parse the `sub` value
    let parsed_values: Vec<&str> = sub.split(',').collect();
    if parsed_values.len() != 3 {
        return HttpResponse::InternalServerError().json(GetCategoriesResponse {
            code: 500,
            message: String::from("Invalid sub format in token"),
            data: vec![],
        });
    }

    // let user_id: &str = parsed_values[0];
    // let role_name: &str = parsed_values[1];
    let shop_id: i32 = parsed_values[2].parse().unwrap();
    match category::get_categories(shop_id, &client).await {
        Ok(categories) => HttpResponse::Ok().json(GetCategoriesResponse {
            code: 200,
            message: String::from("Successful."),
            data: categories,
        }),
        _ => HttpResponse::InternalServerError().json(GetCategoriesResponse {
            code: 500,
            message: String::from("Error trying to read all categories from database"),
            data: vec![],
        }),
    }
}
