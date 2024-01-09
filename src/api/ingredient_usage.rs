use std::sync::Arc;

use actix_web::{post, web, HttpRequest, HttpResponse, Responder};
use tokio::sync::Mutex;
use tokio_postgres::Client;

use crate::{
    models::ingredient_usage::{self, IngredientUsagesRequest},
    utils::{common_struct::BaseResponse, jwt::verify_token_and_get_sub},
};

#[post("/api/ingredient-usages")]
pub async fn add_ingredient_usages(
    req: HttpRequest,
    body: web::Json<IngredientUsagesRequest>,
    data: web::Data<Arc<Mutex<Client>>>,
) -> impl Responder {
    let mut client = data.lock().await;

    let token = match req.headers().get("Authorization") {
        Some(value) => {
            let parts: Vec<&str> = value.to_str().unwrap_or("").split_whitespace().collect();
            if parts.len() == 2 && parts[0] == "Bearer" {
                parts[1]
            } else {
                return HttpResponse::BadRequest().json(BaseResponse {
                    code: 400,
                    message: String::from("Invalid Authorization header format"),
                });
            }
        }
        None => {
            return HttpResponse::Unauthorized().json(BaseResponse {
                code: 401,
                message: String::from("Authorization header missing"),
            })
        }
    };

    let sub = match verify_token_and_get_sub(token) {
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

    // let user_id: &str = parsed_values[0];
    let role: &str = parsed_values[1];
    let shop_id: i32 = parsed_values[2].parse().unwrap();

    if role == "Waiter" {
        return HttpResponse::Unauthorized().json(BaseResponse {
            code: 400,
            message: String::from("Unauthorized!"),
        });
    }

    match ingredient_usage::add_ingredient_usages(&body, shop_id, &mut client).await {
        Ok(is_sufficient) => {
            if !is_sufficient {
                return HttpResponse::BadRequest().json(BaseResponse {
                    code: 400,
                    message: String::from("Insufficient ingredients!"),
                });
            }
            HttpResponse::Ok().json(BaseResponse {
                code: 200,
                message: String::from("Ingredient usages added successfully."),
            })
        }
        Err(err) => {
            println!("{:?}", err);
            HttpResponse::InternalServerError().json(BaseResponse {
                code: 500,
                message: String::from("Error adding ingredient usages to database!"),
            })
        }
    }
}
