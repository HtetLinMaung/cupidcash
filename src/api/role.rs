use std::sync::Arc;

use actix_web::{get, web, HttpRequest, HttpResponse, Responder};
use tokio::sync::Mutex;
use tokio_postgres::Client;

use crate::{
    models::role,
    utils::{
        common_struct::{BaseResponse, DataResponse},
        jwt::verify_token_and_get_sub,
    },
};

#[get("/api/roles")]
pub async fn get_roles(req: HttpRequest, data: web::Data<Arc<Mutex<Client>>>) -> impl Responder {
    let client = data.lock().await;
    // Extract the token from the Authorization header
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

    match role::get_roles(&client).await {
        Ok(roles) => HttpResponse::Ok().json(DataResponse {
            code: 200,
            message: String::from("Roles fetched successfully."),
            data: Some(roles),
        }),
        Err(err) => {
            println!("{:?}", err);
            HttpResponse::InternalServerError().json(BaseResponse {
                code: 500,
                message: String::from("Error fetching roles!"),
            })
        }
    }
}
