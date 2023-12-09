use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use tokio_postgres::{types::ToSql, Client, Error};

use crate::utils::{
    common_struct::PaginationResult,
    sql::{generate_pagination_query, PaginationOptions},
};

#[derive(Debug, Serialize, Deserialize)]
pub struct DiscountType {
    pub id: i32,
    pub description: String,
    pub shop_id: i32,
    pub shop_name: String,
    pub created_at: NaiveDateTime,
}

pub async fn get_discount_types(
    search: &Option<String>,
    page: Option<usize>,
    per_page: Option<usize>,
    role: &str,
    shop_id: i32,
    client: &Client,
) -> Result<PaginationResult<DiscountType>, Error> {
    let mut base_query =
        "from discount_types d join shops s on s.id = d.shop_id where d.deleted_at is null".to_string();
    let mut params: Vec<Box<dyn ToSql + Sync>> = vec![];

    let order_options = if role == "Waiter" {
        "description"
    } else {
        "d.created_at desc"
    };

    if role == "Waiter" {
        params.push(Box::new(shop_id));
        base_query = format!("{base_query} and d.shop_id = ${}", params.len());
    }

    let result = generate_pagination_query(PaginationOptions {
        select_columns: "d.id, d.description, d.shop_id, s.name shop_name, d.created_at",
        base_query: &base_query,
        search_columns: vec!["d.id::varchar", "d.description", "s.name"],
        search: search.as_deref(),
        order_options: Some(&order_options),
        page,
        per_page,
    });

    let params_slice: Vec<&(dyn ToSql + Sync)> = params.iter().map(AsRef::as_ref).collect();

    let row = client.query_one(&result.count_query, &params_slice).await?;
    let total: i64 = row.get("total");

    let mut page_counts = 0;
    let mut current_page = 0;
    let mut limit = 0;
    if page.is_some() && per_page.is_some() {
        current_page = page.unwrap();
        limit = per_page.unwrap();
        page_counts = (total as f64 / limit as f64).ceil() as usize;
    }

    let discount_types: Vec<DiscountType> = client
        .query(&result.query, &params_slice)
        .await?
        .iter()
        .map(|row| DiscountType {
            id: row.get("id"),
            description: row.get("description"),
            shop_id: row.get("shop_id"),
            shop_name: row.get("shop_name"),
            created_at: row.get("created_at"),
        })
        .collect();

    Ok(PaginationResult {
        data: discount_types,
        total,
        page: current_page,
        per_page: limit,
        page_counts,
    })
}

#[derive(Debug, Deserialize)]
pub struct DiscountTypeRequest {
    pub description: String,
    pub shop_id: i32,
}

pub async fn add_discount_type(
    data: &DiscountTypeRequest,
    client: &Client,
) -> Result<(), Box<dyn std::error::Error>> {
    client
        .execute(
            "insert into discount_types (description, shop_id) values ($1, $2)",
            &[&data.description, &data.shop_id],
        )
        .await?;
    Ok(())
}

pub async fn get_discount_type_by_id(discount_type_id: i32, client: &Client) -> Option<DiscountType> {
    let result = client
        .query_one(
            "select d.id, d.description, d.shop_id, s.name shop_name, d.created_at from discount_types d join shops s on s.id = d.shop_id where d.deleted_at is null and d.id = $1",
            &[&discount_type_id],
        )
        .await;

    match result {
        Ok(row) => Some(DiscountType {
            id: row.get("id"),
            description: row.get("description"),
            shop_id: row.get("shop_id"),
            shop_name: row.get("shop_name"),
            created_at: row.get("created_at"),
        }),
        Err(_) => None,
    }
}

pub async fn update_discount_type(
    discount_type_id: i32,
    data: &DiscountTypeRequest,
    client: &Client,
) -> Result<(), Box<dyn std::error::Error>> {
    client
        .execute(
            "update discount_types set description = $1, shop_id = $2 where id = $3",
            &[&data.description, &data.shop_id, &discount_type_id],
        )
        .await?;

    Ok(())
}

pub async fn delete_discount_type(
    discount_type_id: i32,
    client: &Client,
) -> Result<(), Box<dyn std::error::Error>> {
    client
        .execute(
            "update discount_types set deleted_at = CURRENT_TIMESTAMP where id = $1",
            &[&discount_type_id],
        )
        .await?;

    Ok(())
}
