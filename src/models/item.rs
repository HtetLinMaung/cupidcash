use chrono::NaiveDateTime;
use serde::Deserialize;
use std::{fs, option::Option, path::Path};
use tokio_postgres::{types::ToSql, Client, Error};

use crate::utils::{
    common_struct::PaginationResult,
    sql::{generate_pagination_query, PaginationOptions},
};

#[derive(serde::Serialize)]
pub struct ItemCategory {
    pub id: i32,
    pub name: String,
}

#[derive(serde::Serialize)]
pub struct Item {
    pub id: i32,
    pub name: String,
    pub description: String,
    pub price: f64,
    pub image_url: String,
    pub shop_id: i32,
    pub shop_name: String,
    pub created_at: NaiveDateTime,
    pub categories: Vec<ItemCategory>,
}

pub async fn get_items(
    search: &Option<String>,
    page: Option<usize>,
    per_page: Option<usize>,
    shop_id: i32,
    category_id: Option<i32>,
    role: &str,
    client: &Client,
) -> Result<PaginationResult<Item>, Error> {
    let mut base_query =
        "from items i join shops s on i.shop_id = s.id where i.deleted_at is null".to_string();
    let mut params: Vec<Box<dyn ToSql + Sync>> = vec![];

    if role == "Waiter" {
        params.push(Box::new(shop_id));
        base_query = format!("{base_query} and i.shop_id = ${}", params.len());
    }

    if let Some(c) = category_id {
        params.push(Box::new(c));
        base_query = format!("{base_query} and i.item_id in (select item_id from item_categories where category_id = ${})", params.len());
    }

    let order_options = if role == "Waiter" {
        "i.name"
    } else {
        "i.id desc"
    };

    let result = generate_pagination_query(PaginationOptions {
        select_columns: "i.id, i.name, i.description, i.price::text, i.image_url, i.shop_id, s.name shop_name, i.created_at",
        base_query: &base_query,
        search_columns: vec!["i.name", "i.description", "i.price::text", "s.name"],
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

    let rows = client.query(&result.query, &params_slice).await?;

    let mut items: Vec<Item> = vec![];

    for row in &rows {
        let item_id: i32 = row.get("id");

        let category_rows = client.query("select ic.category_id, c.name from item_categories ic join categories c on c.id = ic.category_id where ic.item_id = $1", &[&item_id]).await?;

        let price_str: &str = row.get("price");
        items.push(Item {
            id: item_id,
            name: row.get("name"),
            description: row.get("description"),
            price: price_str.parse().unwrap(),
            image_url: row.get("image_url"),
            shop_id: row.get("shop_id"),
            shop_name: row.get("shop_name"),
            categories: category_rows
                .iter()
                .map(|row| ItemCategory {
                    id: row.get("category_id"),
                    name: row.get("name"),
                })
                .collect(),
            created_at: row.get("created_at"),
        });
    }

    Ok(PaginationResult {
        data: items,
        total,
        page: current_page,
        per_page: limit,
        page_counts,
    })
}

#[derive(Debug, Deserialize)]
pub struct ItemRequest {
    pub name: String,
    pub description: String,
    pub price: f64,
    pub categories: Vec<i32>,
    pub image_url: String,
    pub shop_id: i32,
}

pub async fn add_item(
    data: &ItemRequest,
    client: &Client,
) -> Result<(), Box<dyn std::error::Error>> {
    let query = format!("insert into items (name, description, price, image_url, shop_id) values ($1, $2, {}, $3, $4) returning id", data.price);
    let row = client
        .query_one(
            &query,
            &[
                &data.name,
                &data.description,
                &data.image_url,
                &data.shop_id,
            ],
        )
        .await?;
    let id: i32 = row.get("id");

    for category_id in &data.categories {
        client
            .execute(
                "insert into item_categories (item_id, category_id) values ($1, $2)",
                &[&id, &category_id],
            )
            .await?;
    }

    Ok(())
}

pub async fn get_item_by_id(item_id: i32, client: &Client) -> Option<Item> {
    let result = client
        .query_one(
            "select i.id, i.name, i.description, i.price::text, i.image_url, i.shop_id, s.name shop_name, i.created_at from items i join shops s on i.shop_id = s.id where i.deleted_at is null and i.id = $1",
            &[&item_id],
        )
        .await;

    let category_rows = match client.query("select ic.category_id, c.name from item_categories ic join categories c on c.id = ic.category_id where ic.item_id = $1", &[&item_id]).await {
        Ok(rows) => rows,
        Err(err) => {
            println!("{:?}",err);
            vec![]
        }
    };

    match result {
        Ok(row) => {
            let price_str: &str = row.get("price");
            Some(Item {
                id: item_id,
                name: row.get("name"),
                description: row.get("description"),
                price: price_str.parse().unwrap(),
                image_url: row.get("image_url"),
                shop_id: row.get("shop_id"),
                shop_name: row.get("shop_name"),
                categories: category_rows
                    .iter()
                    .map(|row| ItemCategory {
                        id: row.get("category_id"),
                        name: row.get("name"),
                    })
                    .collect(),
                created_at: row.get("created_at"),
            })
        }
        Err(_) => None,
    }
}

pub async fn update_item(
    item_id: i32,
    old_image_url: &str,
    data: &ItemRequest,
    client: &Client,
) -> Result<(), Box<dyn std::error::Error>> {
    let query = format!("update items set name = $1, description = $2, price = {}, image_url = $3, shop_id = $4 where id = $5", data.price);
    client
        .execute(
            &query,
            &[
                &data.name,
                &data.description,
                &data.image_url,
                &data.shop_id,
                &item_id,
            ],
        )
        .await?;
    client
        .execute(
            "delete from item_categories where item_id = $1",
            &[&item_id],
        )
        .await?;
    for category_id in &data.categories {
        client
            .execute(
                "insert into item_categories (item_id, category_id) values ($1, $2)",
                &[&item_id, &category_id],
            )
            .await?;
    }
    if old_image_url != &data.image_url {
        match fs::remove_file(old_image_url) {
            Ok(_) => println!("File deleted successfully!"),
            Err(e) => println!("Error deleting file: {}", e),
        };
        let path = Path::new(&old_image_url);
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default();
        let extension = path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or_default();
        match fs::remove_file(format!("{stem}_original.{extension}")) {
            Ok(_) => println!("Original file deleted successfully!"),
            Err(e) => println!("Error deleting original file: {}", e),
        };
    }

    Ok(())
}

pub async fn delete_item(
    item_id: i32,
    old_image_url: &str,
    client: &Client,
) -> Result<(), Box<dyn std::error::Error>> {
    client
        .execute(
            "update items set deleted_at = CURRENT_TIMESTAMP where id = $1",
            &[&item_id],
        )
        .await?;
    client
        .execute(
            "delete from item_categories where item_id = $1",
            &[&item_id],
        )
        .await?;
    match fs::remove_file(old_image_url) {
        Ok(_) => println!("File deleted successfully!"),
        Err(e) => println!("Error deleting file: {}", e),
    };
    let path = Path::new(&old_image_url);
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or_default();
    let extension = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or_default();
    match fs::remove_file(format!("{stem}_original.{extension}")) {
        Ok(_) => println!("Original file deleted successfully!"),
        Err(e) => println!("Error deleting original file: {}", e),
    };
    Ok(())
}

// pub async fn get_image_urls(client: &Client) -> Vec<String> {
//     match client.query("select image_url from items", &[]).await {
//         Ok(rows) => rows.iter().map(|row| row.get("image_url")).collect(),
//         Err(err) => {
//             println!("{:?}", err);
//             vec![]
//         }
//     }
// }
