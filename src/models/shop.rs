use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use tokio_postgres::{types::ToSql, Client, Error};

use crate::utils::{
    common_struct::PaginationResult,
    sql::{generate_pagination_query, PaginationOptions},
};

#[derive(Debug, Serialize, Deserialize)]
pub struct Shop {
    pub id: i32,
    pub name: String,
    pub address: String,
    pub created_at: NaiveDateTime,
}

pub async fn get_shops(
    search: &Option<String>,
    page: Option<usize>,
    per_page: Option<usize>,
    client: &Client,
) -> Result<PaginationResult<Shop>, Error> {
    let base_query = "from shops where deleted_at is null".to_string();
    let params: Vec<Box<dyn ToSql + Sync>> = vec![];

    let order_options = "created_at desc";

    let result = generate_pagination_query(PaginationOptions {
        select_columns: "id, name, address, created_at",
        base_query: &base_query,
        search_columns: vec!["id::varchar", "name", "address"],
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

    let shops: Vec<Shop> = client
        .query(&result.query, &params_slice)
        .await?
        .iter()
        .map(|row| Shop {
            id: row.get("id"),
            name: row.get("name"),
            address: row.get("address"),
            created_at: row.get("created_at"),
        })
        .collect();

    Ok(PaginationResult {
        data: shops,
        total,
        page: current_page,
        per_page: limit,
        page_counts,
    })
}

#[derive(Debug, Deserialize)]
pub struct ShopRequest {
    pub name: String,
    pub address: String,
}

pub async fn add_shop(
    data: &ShopRequest,
    client: &Client,
) -> Result<(), Box<dyn std::error::Error>> {
    client
        .execute(
            "insert into shops (name, address) values ($1, $2)",
            &[&data.name, &data.address],
        )
        .await?;
    Ok(())
}

pub async fn get_shop_by_id(shop_id: i32, client: &Client) -> Option<Shop> {
    let result = client
        .query_one(
            "select id, name, address, created_at from shops where id = $1",
            &[&shop_id],
        )
        .await;

    match result {
        Ok(row) => Some(Shop {
            id: row.get("id"),
            name: row.get("name"),
            address: row.get("address"),
            created_at: row.get("created_at"),
        }),
        Err(_) => None,
    }
}

pub async fn update_shop(
    shop_id: i32,
    data: &ShopRequest,
    client: &Client,
) -> Result<(), Box<dyn std::error::Error>> {
    client
        .execute(
            "update shops set name = $1, address = $2 where id = $3",
            &[&data.name, &data.address, &shop_id],
        )
        .await?;

    Ok(())
}

pub async fn delete_shop(shop_id: i32, client: &Client) -> Result<(), Box<dyn std::error::Error>> {
    client
        .execute(
            "update shops set deleted_at = CURRENT_TIMESTAMP where id = $1",
            &[&shop_id],
        )
        .await?;

    Ok(())
}
