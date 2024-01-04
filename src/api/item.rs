use std::sync::Arc;

use actix_web::{delete, get, post, put, web, HttpRequest, HttpResponse, Responder};
use serde::Deserialize;
use tokio::sync::Mutex;
use tokio_postgres::Client;

use crate::{
    models::item::{self, ItemRequest},
    utils::{
        common_struct::{BaseResponse, DataResponse, PaginationResponse},
        jwt::verify_token_and_get_sub,
    },
};

#[derive(Deserialize)]
pub struct GetItemsQuery {
    pub search: Option<String>,
    pub category_id: Option<i32>,
    pub page: Option<usize>,
    pub per_page: Option<usize>,
}

#[get("/api/items")]
pub async fn get_items(
    req: HttpRequest,
    data: web::Data<Arc<Mutex<Client>>>,
    query: web::Query<GetItemsQuery>,
) -> impl Responder {
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

    // let user_id: &str = parsed_values[0];
    let role: &str = parsed_values[1];
    let shop_id: i32 = parsed_values[2].parse().unwrap();
    match item::get_items(
        &query.search,
        query.page,
        query.per_page,
        shop_id,
        query.category_id,
        role,
        &client,
    )
    .await
    {
        Ok(item_result) => HttpResponse::Ok().json(PaginationResponse {
            code: 200,
            message: String::from("Successful."),
            data: item_result.data,
            total: item_result.total,
            page: item_result.page,
            per_page: item_result.per_page,
            page_counts: item_result.page_counts,
        }),
        Err(err) => {
            // Log the error message here
            println!("Error retrieving items: {:?}", err);
            HttpResponse::InternalServerError().json(BaseResponse {
                code: 500,
                message: String::from("Error trying to read all items from database"),
            })
        }
    }
}

#[post("/api/items")]
pub async fn add_item(
    req: HttpRequest,
    body: web::Json<ItemRequest>,
    data: web::Data<Arc<Mutex<Client>>>,
) -> HttpResponse {
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

    // let user_id = parsed_values[0].parse().unwrap();
    let role: &str = parsed_values[1];

    if role == "Waiter" {
        return HttpResponse::Unauthorized().json(BaseResponse {
            code: 401,
            message: String::from("Unauthorized!"),
        });
    }

    if body.name.is_empty() {
        return HttpResponse::BadRequest().json(BaseResponse {
            code: 400,
            message: String::from("Name must not be empty!"),
        });
    }
    if body.description.is_empty() {
        return HttpResponse::BadRequest().json(BaseResponse {
            code: 400,
            message: String::from("Description must not be empty!"),
        });
    }

    if body.price.is_sign_negative() {
        return HttpResponse::BadRequest().json(BaseResponse {
            code: 400,
            message: String::from("Price must not be negative!"),
        });
    }

    if body.discount_type.is_empty() {
        return HttpResponse::BadRequest().json(BaseResponse {
            code: 400,
            message: String::from("Discount type must not be empty!"),
        });
    } else {
        if body.discount_type == "Discount by Specific Percentage" {
            if body.discount_percent.is_sign_negative() {
                return HttpResponse::BadRequest().json(BaseResponse {
                    code: 400,
                    message: String::from("Discount percent must not be negative!"),
                });
            }
            if body.discount_expiration.is_none() {
                return HttpResponse::BadRequest().json(BaseResponse {
                    code: 400,
                    message: String::from("Discount expire data must not be empty!"),
                });
            }
        } else if body.discount_type == "Discount by Specific Amount" {
            if body.discounted_price.is_sign_negative() {
                return HttpResponse::BadRequest().json(BaseResponse {
                    code: 400,
                    message: String::from("Discounted price must not be negative!"),
                });
            }
        }
        if body.discount_type != "No Discount" && body.discount_reason.is_empty() {
            return HttpResponse::BadRequest().json(BaseResponse {
                code: 400,
                message: String::from("Discount reason must not be empty!"),
            });
        }
    }

    match item::add_item(&body, &client).await {
        Ok(()) => HttpResponse::Created().json(BaseResponse {
            code: 201,
            message: String::from("Item added successfully"),
        }),
        Err(e) => {
            eprintln!("Item adding error: {}", e);
            return HttpResponse::InternalServerError().json(BaseResponse {
                code: 500,
                message: String::from("Error adding item!"),
            });
        }
    }
}

#[get("/api/items/{item_id}")]
pub async fn get_item_by_id(
    req: HttpRequest,
    path: web::Path<i32>,
    data: web::Data<Arc<Mutex<Client>>>,
) -> HttpResponse {
    let client = data.lock().await;
    let item_id = path.into_inner();
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

    // let role: &str = parsed_values[1];

    match item::get_item_by_id(item_id, &client).await {
        Some(c) => HttpResponse::Ok().json(DataResponse {
            code: 200,
            message: String::from("Item fetched successfully."),
            data: Some(c),
        }),
        None => HttpResponse::NotFound().json(BaseResponse {
            code: 404,
            message: String::from("Item not found!"),
        }),
    }
}

#[put("/api/items/{item_id}")]
pub async fn update_item(
    req: HttpRequest,
    path: web::Path<i32>,
    body: web::Json<ItemRequest>,
    data: web::Data<Arc<Mutex<Client>>>,
) -> HttpResponse {
    let client = data.lock().await;
    let item_id = path.into_inner();
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

    let role: &str = parsed_values[1];

    if role == "Waiter" {
        return HttpResponse::Unauthorized().json(BaseResponse {
            code: 401,
            message: String::from("Unauthorized!"),
        });
    }

    if body.name.is_empty() {
        return HttpResponse::BadRequest().json(BaseResponse {
            code: 400,
            message: String::from("Name must not be empty!"),
        });
    }
    if body.description.is_empty() {
        return HttpResponse::BadRequest().json(BaseResponse {
            code: 400,
            message: String::from("Description must not be empty!"),
        });
    }

    if body.price.is_sign_negative() {
        return HttpResponse::BadRequest().json(BaseResponse {
            code: 400,
            message: String::from("Price must not be negative!"),
        });
    }

    if body.discount_type.is_empty() {
        return HttpResponse::BadRequest().json(BaseResponse {
            code: 400,
            message: String::from("Discount type must not be empty!"),
        });
    } else {
        if body.discount_type == "Discount by Specific Percentage" {
            if body.discount_percent.is_sign_negative() {
                return HttpResponse::BadRequest().json(BaseResponse {
                    code: 400,
                    message: String::from("Discount percent must not be negative!"),
                });
            }
            if body.discount_expiration.is_none() {
                return HttpResponse::BadRequest().json(BaseResponse {
                    code: 400,
                    message: String::from("Discount expire data must not be empty!"),
                });
            }
        } else if body.discount_type == "Discount by Specific Amount" {
            if body.discounted_price.is_sign_negative() {
                return HttpResponse::BadRequest().json(BaseResponse {
                    code: 400,
                    message: String::from("Discounted price must not be negative!"),
                });
            }
        }
        if body.discount_type != "No Discount" && body.discount_reason.is_empty() {
            return HttpResponse::BadRequest().json(BaseResponse {
                code: 400,
                message: String::from("Discount reason must not be empty!"),
            });
        }
    }

    match item::get_item_by_id(item_id, &client).await {
        Some(i) => match item::update_item(item_id, &i.image_url, &body, &client).await {
            Ok(()) => HttpResponse::Ok().json(BaseResponse {
                code: 200,
                message: String::from("Item updated successfully"),
            }),
            Err(e) => {
                eprintln!("Item updating error: {}", e);
                return HttpResponse::InternalServerError().json(BaseResponse {
                    code: 500,
                    message: String::from("Error updating item!"),
                });
            }
        },
        None => HttpResponse::NotFound().json(BaseResponse {
            code: 404,
            message: String::from("Item not found!"),
        }),
    }
}

#[delete("/api/items/{item_id}")]
pub async fn delete_item(
    req: HttpRequest,
    path: web::Path<i32>,
    data: web::Data<Arc<Mutex<Client>>>,
) -> HttpResponse {
    let client = data.lock().await;
    let item_id = path.into_inner();
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

    let role: &str = parsed_values[1];

    if role == "Waiter" {
        return HttpResponse::Unauthorized().json(BaseResponse {
            code: 401,
            message: String::from("Unauthorized!"),
        });
    }

    match item::get_item_by_id(item_id, &client).await {
        Some(i) => match item::delete_item(item_id, &i.image_url, &client).await {
            Ok(()) => HttpResponse::Ok().json(BaseResponse {
                code: 204,
                message: String::from("Item deleted successfully"),
            }),
            Err(e) => {
                eprintln!("Item deleting error: {}", e);
                return HttpResponse::InternalServerError().json(BaseResponse {
                    code: 500,
                    message: String::from("Error deleting item!"),
                });
            }
        },
        None => HttpResponse::NotFound().json(BaseResponse {
            code: 404,
            message: String::from("Item not found!"),
        }),
    }
}
