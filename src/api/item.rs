use std::sync::Arc;

use actix_web::{get, web, HttpRequest, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use tokio_postgres::Client;

use crate::{
    models::item::{self, Item},
    utils::jwt::verify_token_and_get_sub,
};

#[derive(Serialize)]
pub struct GetItemsResponse {
    pub code: u16,
    pub message: String,
    pub data: Vec<Item>,
}

#[derive(Deserialize)]
pub struct GetItemsQuery {
    pub search: Option<String>,
    pub category_id: Option<i32>,
}

#[get("/api/items")]
pub async fn get_items(
    req: HttpRequest,
    client: web::Data<Arc<Client>>,
    query: web::Query<GetItemsQuery>,
) -> impl Responder {
    // Extract the token from the Authorization header
    let token = match req.headers().get("Authorization") {
        Some(value) => {
            let parts: Vec<&str> = value.to_str().unwrap_or("").split_whitespace().collect();
            if parts.len() == 2 && parts[0] == "Bearer" {
                parts[1]
            } else {
                return HttpResponse::BadRequest().json(GetItemsResponse {
                    code: 400,
                    message: String::from("Invalid Authorization header format"),
                    data: vec![],
                });
            }
        }
        None => {
            return HttpResponse::Unauthorized().json(GetItemsResponse {
                code: 401,
                message: String::from("Authorization header missing"),
                data: vec![],
            })
        }
    };

    let sub = match verify_token_and_get_sub(token) {
        Some(s) => s,
        None => {
            return HttpResponse::Unauthorized().json(GetItemsResponse {
                code: 401,
                message: String::from("Invalid token"),
                data: vec![],
            })
        }
    };

    // Parse the `sub` value
    let parsed_values: Vec<&str> = sub.split(',').collect();
    if parsed_values.len() != 3 {
        return HttpResponse::InternalServerError().json(GetItemsResponse {
            code: 500,
            message: String::from("Invalid sub format in token"),
            data: vec![],
        });
    }

    // let user_id: &str = parsed_values[0];
    // let role_name: &str = parsed_values[1];
    let shop_id: i32 = parsed_values[2].parse().unwrap();
    match item::get_items(shop_id, &query.search, &query.category_id, &client).await {
        Ok(items) => HttpResponse::Ok().json(GetItemsResponse {
            code: 200,
            message: String::from("Successful."),
            data: items,
        }),
        _ => HttpResponse::InternalServerError().json(GetItemsResponse {
            code: 500,
            message: String::from("Error trying to read all items from database"),
            data: vec![],
        }),
    }
}
