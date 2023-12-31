use actix_web::{get, post, put, web, HttpRequest, HttpResponse, Responder};
use chrono::NaiveDate;
use serde::Deserialize;
use serde_json::Value;
use std::{collections::HashMap, fs, io, sync::Arc};
use tokio::sync::Mutex;
use tokio_postgres::Client;

use crate::{
    models::order::{self, NewOrder},
    models::shop::{self},
    utils::{
        common_struct::{BaseResponse, DataResponse, PaginationResponse},
        jwt::verify_token_and_get_sub,
        socketio,
    },
};

#[post("/api/orders")]
pub async fn create_order(
    req: HttpRequest,
    body: web::Json<NewOrder>,
    data: web::Data<Arc<Mutex<Client>>>,
) -> impl Responder {
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

    let user_id: i32 = parsed_values[0].parse().unwrap();
    // let role_name: &str = parsed_values[1];
    // let shop_id: i32 = parsed_values[2].parse().unwrap();
    match order::order_exists_in_table(&body.table_id, &client).await {
        Ok(exists) => {
            if exists {
                return HttpResponse::BadRequest().json(BaseResponse {
                    code: 400,
                    message: String::from("Order already exists in the request table!"),
                });
            }
            match order::create_order(user_id, body.into_inner(), &mut client).await {
                Ok(id) => {
                    tokio::spawn(async move {
                        let mut payload: HashMap<String, Value> = HashMap::new();
                        payload.insert("order_id".to_string(), Value::Number(id.into()));
                        match socketio::emit("/pos", "new-order", &vec![], Some(payload)).await {
                            Ok(_) => {
                                println!("new-order event sent successfully.");
                            }
                            Err(err) => {
                                println!("{:?}", err);
                            }
                        };
                    });
                    HttpResponse::Ok().json(DataResponse {
                        code: 200,
                        message: String::from("Order created successfully"),
                        data: Some(id),
                    })
                }
                Err(err) => HttpResponse::InternalServerError().json(BaseResponse {
                    code: 400,
                    message: err.to_string(),
                }),
              
            }
        }
        Err(e) => {
            eprintln!("Database error: {}", e);
            return HttpResponse::InternalServerError().json(BaseResponse {
                code: 500,
                message: String::from("Something went wrong!"),
            });
        }
    }
}

#[derive(Deserialize)]
pub struct GetOrdersQuery {
    pub search: Option<String>,
    pub page: Option<usize>,
    pub per_page: Option<usize>,
    pub from_date: Option<NaiveDate>,
    pub to_date: Option<NaiveDate>,
    pub status: Option<String>,
}

#[get("/api/orders")]
pub async fn get_orders(
    req: HttpRequest,
    query: web::Query<GetOrdersQuery>,
    data: web::Data<Arc<Mutex<Client>>>,
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

    let user_id: i32 = parsed_values[0].parse().unwrap();
    let role: &str = parsed_values[1];
    let shop_id: i32 = parsed_values[2].parse().unwrap();

    match order::get_orders(
        &query.search,
        query.page,
        query.per_page,
        shop_id,
        user_id,
        role,
        &query.from_date,
        &query.to_date,
        &query.status,
        &client,
    )
    .await
    {
        Ok(order_result) => HttpResponse::Ok().json(PaginationResponse {
            code: 200,
            message: String::from("Successful."),
            data: order_result.data,
            total: order_result.total,
            page: order_result.page,
            per_page: order_result.per_page,
            page_counts: order_result.page_counts,
        }),
        Err(err) => {
            // Log the error message here
            println!("Error retrieving orders: {:?}", err);
            HttpResponse::InternalServerError().json(BaseResponse {
                code: 500,
                message: String::from("Error trying to read all orders from database"),
            })
        }
    }
}

#[get("/api/orders/{order_id}/details")]
pub async fn get_order_detail(
    req: HttpRequest,
    order_id: web::Path<i32>,
    data: web::Data<Arc<Mutex<Client>>>,
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

    let user_id: i32 = parsed_values[0].parse().unwrap();
    let role_name: &str = parsed_values[1];
    let shop_id: i32 = parsed_values[2].parse().unwrap();
    let order_id = order_id.into_inner(); // Extract the inner value
    match order::get_order_detail(shop_id, user_id, order_id, role_name, &client).await {
        Ok(order_detail) => HttpResponse::Ok().json(DataResponse {
            code: 200,
            message: String::from("Successful."),
            data: Some(order_detail),
        }),
        Err(err) => {
            println!("{:?}", err);
            HttpResponse::InternalServerError().json(BaseResponse {
                code: 500,
                message: String::from("Error trying to read order details from database"),
            })
        }
    }
}

#[get("/api/orders/{order_id}")]
pub async fn get_order_by_id(
    req: HttpRequest,
    path: web::Path<i32>,
    data: web::Data<Arc<Mutex<Client>>>,
) -> HttpResponse {
    let client = data.lock().await;
    let order_id = path.into_inner();
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

    let user_id: i32 = parsed_values[0].parse().unwrap();
    let role: &str = parsed_values[1];
    let shop_id: i32 = parsed_values[2].parse().unwrap();

    match order::get_order_by_id(order_id, user_id, shop_id, role, &client).await {
        Some(c) => HttpResponse::Ok().json(DataResponse {
            code: 200,
            message: String::from("Order fetched successfully."),
            data: Some(c),
        }),
        None => HttpResponse::NotFound().json(BaseResponse {
            code: 404,
            message: String::from("Order not found!"),
        }),
    }
}

#[derive(Deserialize)]
pub struct UpdateOrderRequest {
    pub status: String,
    pub tax: Option<f64>,
    pub discount: Option<f64>,
}

#[put("/api/orders/{order_id}")]
pub async fn update_order(
    req: HttpRequest,
    path: web::Path<i32>,
    body: web::Json<UpdateOrderRequest>,
    data: web::Data<Arc<Mutex<Client>>>,
) -> HttpResponse {
    let client = data.lock().await;
    let order_id = path.into_inner();
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

    let user_id: i32 = parsed_values[0].parse().unwrap();
    let role: &str = parsed_values[1];
    let shop_id: i32 = parsed_values[2].parse().unwrap();

    let status_list: Vec<&str> = vec!["Pending", "Served", "Canceled", "Completed"];
    if !status_list.contains(&body.status.as_str()) {
        return HttpResponse::BadRequest().json(BaseResponse {
            code: 400,
            message: String::from(
                "Please select a valid status: Pending, Served, Canceled, or Completed.",
            ),
        });
    }

    if role == "Waiter" {
        if &body.status != "Canceled" {
            return HttpResponse::Unauthorized().json(BaseResponse {
                code: 401,
                message: String::from("Unauthorized!"),
            });
        }
    }

    match order::get_order_by_id(order_id, user_id, shop_id, role, &client).await {
        Some(o) => {
            let mut tax = o.tax;
            let mut discount = o.discount;
            if &body.status == "Completed" {
                if let Some(t) = body.tax {
                    tax = t;
                }
                if let Some(d) = body.discount {
                    discount = d;
                }
            }

            match order::update_order(order_id, &body.status, tax, discount, &client).await {
                Ok(()) => HttpResponse::Ok().json(BaseResponse {
                    code: 200,
                    message: String::from("Order updated successfully"),
                }),
                Err(e) => {
                    eprintln!("Order updating error: {}", e);
                    return HttpResponse::InternalServerError().json(BaseResponse {
                        code: 500,
                        message: String::from("Error updating order!"),
                    });
                }
            }
        }
        None => HttpResponse::NotFound().json(BaseResponse {
            code: 404,
            message: String::from("Order not found!"),
        }),
    }
}

#[derive(Deserialize)]
pub struct ReportQuery {
    pub from_date: NaiveDate,
    pub to_date: NaiveDate,
    pub shop_id: i32,
}
#[get("/api/daily-sale-report")]
pub async fn get_daily_sale_report(
    req: HttpRequest,
    query: web::Query<ReportQuery>,
    data: web::Data<Arc<Mutex<Client>>>,
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

    if query.from_date.gt(&query.to_date) {
        return HttpResponse::BadRequest().json(BaseResponse {
            code: 400,
            message: String::from("Invalid Date Range!"),
        });
    }


    let user_id: i32 = parsed_values[0].parse().unwrap();

    match shop::get_shop_by_id(query.shop_id, &client).await {
        Some(s) => {
            match order::get_daily_sale_report(
                query.from_date,
                query.to_date,
                query.shop_id,
                s.name,
                user_id,
                &client,
            )
            .await
            {
                Ok(data) => HttpResponse::Ok().json(DataResponse {
                    code: 200,
                    message: String::from("Successful."),
                    data: Some(data),
                }),
                Err(err) => {
                    // Log the error message here
                    println!("Error retrieving orders: {:?}", err);
                    HttpResponse::InternalServerError().json(BaseResponse {
                        code: 500,
                        message: String::from("Error trying to read all orders from database"),
                    })
                }
            }
        },
        None => HttpResponse::NotFound().json(BaseResponse {
            code: 404,
            message: String::from("Shop not found!"),
        }),
    }
}

#[get("/api/daily-sale-report-pdf")]
pub async fn daily_sale_report_pdf(
    req: HttpRequest,
) -> Result<HttpResponse, Box<dyn std::error::Error>> {
    // Extract the token from the Authorization header
    let token = match req.headers().get("Authorization") {
        Some(value) => {
            let parts: Vec<&str> = value.to_str().unwrap_or("").split_whitespace().collect();
            if parts.len() == 2 && parts[0] == "Bearer" {
                parts[1]
            } else {
                return Ok(HttpResponse::BadRequest().json(BaseResponse {
                    code: 400,
                    message: String::from("Invalid Authorization header format"),
                }));
            }
        }
        None => {
            return Ok(HttpResponse::Unauthorized().json(BaseResponse {
                code: 401,
                message: String::from("Authorization header missing"),
            }))
        }
    };

    let sub = match verify_token_and_get_sub(token) {
        Some(s) => s,
        None => {
            return Ok(HttpResponse::Unauthorized().json(BaseResponse {
                code: 401,
                message: String::from("Invalid token"),
            }))
        }
    };

    // Parse the `sub` value
    let parsed_values: Vec<&str> = sub.split(',').collect();
    if parsed_values.len() != 3 {
        return Ok(HttpResponse::InternalServerError().json(BaseResponse {
            code: 500,
            message: String::from("Invalid sub format in token"),
        }));
    }
    let user_id: i32 = parsed_values[0].parse().unwrap();
    // Assuming you have the dynamically determined path to the PDF file
    let file_path = format!("reports/{}dailysalereport.pdf", user_id); // Replace this with your dynamic path logic

    // Read the file content
    let file_content = fs::read(&file_path).map_err(|e| {
        println!("Error reading PDF file: {:?}", e);
        Box::new(io::Error::new(io::ErrorKind::Other, e)) as Box<dyn std::error::Error>
    })?;

    // Delete the file after it has been downloaded
    if let Err(err) = fs::remove_file(&file_path) {
        println!("Error deleting Excel file: {:?}", err);
    }
    // Serve the PDF file
    Ok(HttpResponse::Ok()
        .append_header((
            "Content-Disposition",
            "attachment; filename=dailysalereport.pdf",
        ))
        .content_type("application/pdf")
        .body(file_content))
}
#[get("/api/daily-sale-report-excel")]
pub async fn daily_sale_report_excel(
    req: HttpRequest,
) -> Result<HttpResponse, Box<dyn std::error::Error>> {
    // Extract the token from the Authorization header
    let token = match req.headers().get("Authorization") {
        Some(value) => {
            let parts: Vec<&str> = value.to_str().unwrap_or("").split_whitespace().collect();
            if parts.len() == 2 && parts[0] == "Bearer" {
                parts[1]
            } else {
                return Ok(HttpResponse::BadRequest().json(BaseResponse {
                    code: 400,
                    message: String::from("Invalid Authorization header format"),
                }));
            }
        }
        None => {
            return Ok(HttpResponse::Unauthorized().json(BaseResponse {
                code: 401,
                message: String::from("Authorization header missing"),
            }))
        }
    };

    let sub = match verify_token_and_get_sub(token) {
        Some(s) => s,
        None => {
            return Ok(HttpResponse::Unauthorized().json(BaseResponse {
                code: 401,
                message: String::from("Invalid token"),
            }))
        }
    };

    // Parse the `sub` value
    let parsed_values: Vec<&str> = sub.split(',').collect();
    if parsed_values.len() != 3 {
        return Ok(HttpResponse::InternalServerError().json(BaseResponse {
            code: 500,
            message: String::from("Invalid sub format in token"),
        }));
    }
    let user_id: i32 = parsed_values[0].parse().unwrap();
    // Assuming you have the dynamically determined path to the Excel file
    let file_path = format!("reports/{}dailysalereport.xlsx", user_id); // Replace this with your dynamic path logic

    // Read the file content
    let file_content = fs::read(&file_path).map_err(|e| {
        println!("Error reading Excel file: {:?}", e);
        Box::new(io::Error::new(io::ErrorKind::Other, e)) as Box<dyn std::error::Error>
    })?;

    // Serve the Excel file
    let response = HttpResponse::Ok()
        .append_header((
            "Content-Disposition",
            "attachment; filename=dailysalereport.xlsx",
        ))
        .content_type("application/vnd.openxmlformats-officedocument.spreadsheetml.sheet")
        .body(file_content);

    // Delete the file after it has been downloaded
    if let Err(err) = fs::remove_file(&file_path) {
        println!("Error deleting Excel file: {:?}", err);
    }

    Ok(response)
}

