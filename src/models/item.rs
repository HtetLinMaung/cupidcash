use std::option::Option;
use tokio_postgres::{types::ToSql, Client, Error};

use crate::utils::{
    common_struct::PaginationResult,
    sql::{generate_pagination_query, PaginationOptions},
};

#[derive(serde::Serialize)]
pub struct Item {
    pub id: i32,
    pub name: String,
    pub description: String,
    pub price: f64,
    pub image_url: String,
}

pub struct GetItemsResult {
    pub items: Vec<Item>,
    pub total: i64,
    pub page: u32,
    pub per_page: u32,
    pub page_counts: usize,
}

pub async fn get_items(
    search: &Option<String>,
    page: Option<usize>,
    per_page: Option<usize>,
    shop_id: i32,
    category_id: Option<i32>,
    client: &Client,
) -> Result<PaginationResult<Item>, Error> {
    let mut base_query = "from items where shop_id = $1 and deleted_at is null".to_string();
    let mut params: Vec<Box<dyn ToSql + Sync>> = vec![Box::new(shop_id)];

    if let Some(c) = category_id {
        params.push(Box::new(c));
        base_query = format!("{base_query} and category_id = ${}", params.len());
    }

    let order_options = "name, id desc";

    let result = generate_pagination_query(PaginationOptions {
        select_columns: "id, name, description, price::text, image_url",
        base_query: &base_query,
        search_columns: vec!["name"],
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

    let items: Vec<Item> = client
        .query(&result.query, &params_slice)
        .await?
        .iter()
        .map(|row| {
            let price_str: &str = row.get("price");

            Item {
                id: row.get("id"),
                name: row.get("name"),
                description: row.get("description"),
                price: price_str.parse().unwrap(),
                image_url: row.get("image_url"),
            }
        })
        .collect();

    Ok(PaginationResult {
        data: items,
        total,
        page: current_page,
        per_page: limit,
        page_counts,
    })
}
