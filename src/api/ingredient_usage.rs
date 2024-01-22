use std::sync::Arc;

use actix_web::{delete, get, post, put, web, HttpRequest, HttpResponse, Responder};
use serde::Deserialize;
use tokio::sync::Mutex;
use tokio_postgres::Client;

use crate::{
    models::ingredient_usage::{self, IngredientUsagesRequest},
    utils::{
        common_struct::{BaseResponse, DataResponse, PaginationResponse},
        jwt::verify_token_and_get_sub,
    },
};

#[derive(Deserialize)]
pub struct GetIngredientUsagesQuery {
    pub search: Option<String>,
    pub page: Option<usize>,
    pub per_page: Option<usize>,
}

#[get("/api/ingredient-usages")]
pub async fn get_ingredient_usages(
    req: HttpRequest,
    data: web::Data<Arc<Mutex<Client>>>,
    query: web::Query<GetIngredientUsagesQuery>,
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

    match ingredient_usage::get_ingredient_usages(
        &query.search,
        query.page,
        query.per_page,
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
            println!("Error retrieving ingredient_usages: {:?}", err);
            HttpResponse::InternalServerError().json(BaseResponse {
                code: 500,
                message: String::from("Error trying to read all ingredient_usages from database"),
            })
        }
    }
}

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
    // let shop_id: i32 = parsed_values[2].parse().unwrap();

    if role == "Waiter" {
        return HttpResponse::Unauthorized().json(BaseResponse {
            code: 400,
            message: String::from("Unauthorized!"),
        });
    }

    if body.shop_id.is_none() {
        return HttpResponse::BadRequest().json(BaseResponse {
            code: 400,
            message: String::from("Shop Id must not be empty!"),
        });
    }

    for ingredient_usage in &body.ingredient_usages {
        if ingredient_usage.ingredient_id.is_none() || ingredient_usage.ingredient_id.unwrap() == 0
        {
            return HttpResponse::BadRequest().json(BaseResponse {
                code: 400,
                message: String::from("Ingredient ID must not be empty!"),
            });
        }

        if ingredient_usage.quantity_used.is_none()
            || ingredient_usage.quantity_used.unwrap() <= 0.0
        {
            return HttpResponse::BadRequest().json(BaseResponse {
                code: 400,
                message: String::from(
                    "Quantity Used must not be empty or less than or equal to 0.0!",
                ),
            });
        }
    }

    match ingredient_usage::add_ingredient_usages(&body, &mut client).await {
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

#[get("/api/ingredient-usages/{ingredient_usage_id}")]
pub async fn get_ingredient_usage_by_id(
    req: HttpRequest,
    path: web::Path<i32>,
    data: web::Data<Arc<Mutex<Client>>>,
) -> HttpResponse {
    let client = data.lock().await;
    let ingredient_usage_id = path.into_inner();
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

    if role != "Admin" && role != "Manager" {
        return HttpResponse::Unauthorized().json(BaseResponse {
            code: 401,
            message: String::from("Unauthorized!"),
        });
    }

    match ingredient_usage::get_ingredient_usage_by_id(ingredient_usage_id, &client).await {
        Some(c) => HttpResponse::Ok().json(DataResponse {
            code: 200,
            message: String::from("IngredientUsages fetched successfully."),
            data: Some(c),
        }),
        None => HttpResponse::NotFound().json(BaseResponse {
            code: 404,
            message: String::from("IngredientUsages not found!"),
        }),
    }
}

#[put("/api/ingredient-usages")]
pub async fn update_ingredient_usage(
    req: HttpRequest,
    body: web::Json<IngredientUsagesRequest>,
    data: web::Data<Arc<Mutex<Client>>>,
) -> HttpResponse {
    let mut client = data.lock().await;
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
            code: 400,
            message: String::from("Unauthorized!"),
        });
    }

    for ingredient_usage in &body.ingredient_usages {
        if ingredient_usage.usage_id.is_none() || ingredient_usage.usage_id.unwrap() == 0
        {
            return HttpResponse::BadRequest().json(BaseResponse {
                code: 400,
                message: String::from("Usage ID must not be empty!"),
            });
        }
        match ingredient_usage::get_ingredient_usage_by_id(ingredient_usage.usage_id.unwrap(), &client).await
        {
            Some(iur_db) => {
                if iur_db.ingredient_id != ingredient_usage.ingredient_id.unwrap() {
                    return HttpResponse::BadRequest().json(BaseResponse {
                        code: 400,
                        message: String::from("Ingredient ID must not be changed!"),
                    });
                }
            }
            None => {
                return HttpResponse::NotFound().json(BaseResponse {
                    code: 404,
                    message: String::from("IngredientUsages not found!"),
                });
            }
        }
        if ingredient_usage.ingredient_id.is_none() || ingredient_usage.ingredient_id.unwrap() == 0
        {
            return HttpResponse::BadRequest().json(BaseResponse {
                code: 400,
                message: String::from("Ingredient ID must not be empty!"),
            });
        }

        if ingredient_usage.quantity_used.is_none()
            || ingredient_usage.quantity_used.unwrap() <= 0.0
        {
            return HttpResponse::BadRequest().json(BaseResponse {
                code: 400,
                message: String::from(
                    "Quantity Used must not be empty or less than or equal to 0.0!",
                ),
            });
        }
    }

    match ingredient_usage::update_ingredient_usage(&body, &mut client).await {
        Ok(is_sufficient) => {
            if !is_sufficient {
                return HttpResponse::BadRequest().json(BaseResponse {
                    code: 400,
                    message: String::from("Insufficient ingredients!"),
                });
            }
            HttpResponse::Ok().json(BaseResponse {
                code: 200,
                message: String::from("Ingredient usages updated successfully."),
            })
        }
        Err(err) => {
            println!("{:?}", err);
            HttpResponse::InternalServerError().json(BaseResponse {
                code: 500,
                message: String::from("Error updating ingredient usages to database!"),
            })
        }
    }
}

#[delete("/api/ingredient-usages/{ingredient_usage_id}")]
pub async fn delete_ingredient_usage(
    req: HttpRequest,
    path: web::Path<i32>,
    data: web::Data<Arc<Mutex<Client>>>,
) -> HttpResponse {
    let client = data.lock().await;
    let ingredient_usage_id = path.into_inner();
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

    if role != "Admin" && role != "Manager" {
        return HttpResponse::Unauthorized().json(BaseResponse {
            code: 401,
            message: String::from("Unauthorized!"),
        });
    }

    match ingredient_usage::get_ingredient_usage_by_id(ingredient_usage_id, &client).await {
        Some(iur_db) => match ingredient_usage::delete_ingredient_usage(
            ingredient_usage_id,
            iur_db.ingredient_id,
            iur_db.quantity_used,
            &client,
        )
        .await
        {
            Ok(()) => HttpResponse::Ok().json(BaseResponse {
                code: 204,
                message: String::from("IngredientUsages deleted successfully"),
            }),
            Err(e) => {
                eprintln!("IngredientUsages deleting error: {}", e);
                return HttpResponse::InternalServerError().json(BaseResponse {
                    code: 500,
                    message: String::from("Error deleting ingredient_usage!"),
                });
            }
        },
        None => HttpResponse::NotFound().json(BaseResponse {
            code: 404,
            message: String::from("IngredientUsages not found!"),
        }),
    }
}
