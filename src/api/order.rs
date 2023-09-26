use actix_web::{get, post, web, HttpRequest, HttpResponse, Responder};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::{sync::Arc, vec};
use tokio_postgres::Client;

use crate::{
    models::order::{self, NewOrder, Order, OrderDetail},
    utils::jwt::verify_token_and_get_sub,
};

#[derive(Serialize)]
pub struct CreateOrderResponse {
    pub code: u16,
    pub message: String,
}

#[post("/api/orders")]
pub async fn create_order(
    req: HttpRequest,
    data: web::Json<NewOrder>,
    client: web::Data<Arc<Client>>,
) -> impl Responder {
    // Extract the token from the Authorization header
    let token = match req.headers().get("Authorization") {
        Some(value) => {
            let parts: Vec<&str> = value.to_str().unwrap_or("").split_whitespace().collect();
            if parts.len() == 2 && parts[0] == "Bearer" {
                parts[1]
            } else {
                return HttpResponse::BadRequest().json(CreateOrderResponse {
                    code: 400,
                    message: String::from("Invalid Authorization header format"),
                });
            }
        }
        None => {
            return HttpResponse::Unauthorized().json(CreateOrderResponse {
                code: 401,
                message: String::from("Authorization header missing"),
            })
        }
    };

    let sub = match verify_token_and_get_sub(token) {
        Some(s) => s,
        None => {
            return HttpResponse::Unauthorized().json(CreateOrderResponse {
                code: 401,
                message: String::from("Invalid token"),
            })
        }
    };

    // Parse the `sub` value
    let parsed_values: Vec<&str> = sub.split(',').collect();
    if parsed_values.len() != 3 {
        return HttpResponse::InternalServerError().json(CreateOrderResponse {
            code: 500,
            message: String::from("Invalid sub format in token"),
        });
    }

    let user_id: i32 = parsed_values[0].parse().unwrap();
    // let role_name: &str = parsed_values[1];
    // let shop_id: i32 = parsed_values[2].parse().unwrap();

    match order::create_order(user_id, data.into_inner(), &client).await {
        Ok(_) => HttpResponse::Ok().json(CreateOrderResponse {
            code: 200,
            message: String::from("Order created successfully"),
        }),
        Err(_) => HttpResponse::InternalServerError().json(CreateOrderResponse {
            code: 500,
            message: String::from("Error creating order"),
        }),
    }
}

#[derive(Serialize)]
pub struct GetOrderResponse {
    pub code: u16,
    pub message: String,
    pub data: Vec<Order>,
    pub total: i64,
    pub page: u32,
    pub per_page: u32,
    pub page_counts: usize,
}

#[derive(Deserialize)]
pub struct GetOrdersQuery {
    pub page: Option<u32>,
    pub per_page: Option<u32>,
    pub from_date: Option<NaiveDate>,
    pub to_date: Option<NaiveDate>,
}

#[get("/api/orders")]
pub async fn get_orders(
    req: HttpRequest,
    query: web::Query<GetOrdersQuery>,
    client: web::Data<Arc<Client>>,
) -> impl Responder {
    // Extract the token from the Authorization header
    let token = match req.headers().get("Authorization") {
        Some(value) => {
            let parts: Vec<&str> = value.to_str().unwrap_or("").split_whitespace().collect();
            if parts.len() == 2 && parts[0] == "Bearer" {
                parts[1]
            } else {
                return HttpResponse::BadRequest().json(GetOrderResponse {
                    code: 400,
                    message: String::from("Invalid Authorization header format"),
                    data: vec![],
                    total: 0,
                    page: 0,
                    per_page: 0,
                    page_counts: 0,
                });
            }
        }
        None => {
            return HttpResponse::Unauthorized().json(GetOrderResponse {
                code: 401,
                message: String::from("Authorization header missing"),
                data: vec![],
                total: 0,
                page: 0,
                per_page: 0,
                page_counts: 0,
            })
        }
    };

    let sub = match verify_token_and_get_sub(token) {
        Some(s) => s,
        None => {
            return HttpResponse::Unauthorized().json(GetOrderResponse {
                code: 401,
                message: String::from("Invalid token"),
                data: vec![],
                total: 0,
                page: 0,
                per_page: 0,
                page_counts: 0,
            })
        }
    };

    // Parse the `sub` value
    let parsed_values: Vec<&str> = sub.split(',').collect();
    if parsed_values.len() != 3 {
        return HttpResponse::InternalServerError().json(GetOrderResponse {
            code: 500,
            message: String::from("Invalid sub format in token"),
            data: vec![],
            total: 0,
            page: 0,
            per_page: 0,
            page_counts: 0,
        });
    }

    let user_id: i32 = parsed_values[0].parse().unwrap();
    let role_name: &str = parsed_values[1];
    let shop_id: i32 = parsed_values[2].parse().unwrap();

    match order::get_orders(
        shop_id,
        user_id,
        role_name,
        &query.page,
        &query.per_page,
        &query.from_date,
        &query.to_date,
        &client,
    )
    .await
    {
        Ok(order_result) => HttpResponse::Ok().json(GetOrderResponse {
            code: 200,
            message: String::from("Successful."),
            data: order_result.orders,
            total: order_result.total,
            page: order_result.page,
            per_page: order_result.per_page,
            page_counts: order_result.page_counts,
        }),
        Err(err) => {
            // Log the error message here
            println!("Error retrieving orders: {:?}", err);
            HttpResponse::InternalServerError().json(GetOrderResponse {
                code: 500,
                message: String::from("Error trying to read all orders from database"),
                data: vec![],
                total: 0,
                page: 0,
                per_page: 0,
                page_counts: 0,
            })
        }
    }
}

#[derive(Serialize)]
pub struct GetOrderDetailsResponse {
    pub code: u16,
    pub message: String,
    pub data: Option<OrderDetail>,
}

#[get("/api/orders/{order_id}/details")]
pub async fn get_order_detail(
    req: HttpRequest,
    order_id: web::Path<i32>,
    client: web::Data<Arc<Client>>,
) -> impl Responder {
    // Extract the token from the Authorization header
    let token = match req.headers().get("Authorization") {
        Some(value) => {
            let parts: Vec<&str> = value.to_str().unwrap_or("").split_whitespace().collect();
            if parts.len() == 2 && parts[0] == "Bearer" {
                parts[1]
            } else {
                return HttpResponse::BadRequest().json(GetOrderDetailsResponse {
                    code: 400,
                    message: String::from("Invalid Authorization header format"),
                    data: None,
                });
            }
        }
        None => {
            return HttpResponse::Unauthorized().json(GetOrderDetailsResponse {
                code: 401,
                message: String::from("Authorization header missing"),
                data: None,
            })
        }
    };

    let sub = match verify_token_and_get_sub(token) {
        Some(s) => s,
        None => {
            return HttpResponse::Unauthorized().json(GetOrderDetailsResponse {
                code: 401,
                message: String::from("Invalid token"),
                data: None,
            })
        }
    };

    // Parse the `sub` value
    let parsed_values: Vec<&str> = sub.split(',').collect();
    if parsed_values.len() != 3 {
        return HttpResponse::InternalServerError().json(GetOrderDetailsResponse {
            code: 500,
            message: String::from("Invalid sub format in token"),
            data: None,
        });
    }

    let user_id: i32 = parsed_values[0].parse().unwrap();
    // let role_name: &str = parsed_values[1];
    let shop_id: i32 = parsed_values[2].parse().unwrap();
    let order_id = order_id.into_inner(); // Extract the inner value
    match order::get_order_detail(shop_id, user_id, order_id, &client).await {
        Ok(order_detail) => HttpResponse::Ok().json(GetOrderDetailsResponse {
            code: 200,
            message: String::from("Successful."),
            data: Some(order_detail),
        }),
        Err(_) => HttpResponse::InternalServerError().json(GetOrderDetailsResponse {
            code: 500,
            message: String::from("Error trying to read order details from database"),
            data: None,
        }),
    }
}
