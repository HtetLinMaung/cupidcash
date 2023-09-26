use std::option::Option;
use tokio_postgres::{types::ToSql, Client, Error};

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
    shop_id: i32,
    name: &Option<String>,
    category_id: &Option<i32>,
    page: &Option<u32>,
    per_page: &Option<u32>,
    client: &Client,
) -> Result<GetItemsResult, Error> {
    let mut query =
        "select id, name, description, price::text, image_url from items where shop_id = $1 and deleted_at is null".to_string();
    let mut count_sql = String::from(
        "select count(*) as total from items where shop_id = $1 and deleted_at is null",
    );
    let mut params: Vec<Box<dyn ToSql + Sync>> = vec![Box::new(shop_id)];
    if let Some(n) = name {
        query = format!("{query} and name like '%{n}%'");
        count_sql = format!("{count_sql} and name like '%{n}%'");
    }

    if let Some(c) = category_id {
        query = format!("{} and category_id = $2", query);
        count_sql = format!("{count_sql} and category_id = $2");
        params.push(Box::new(c));
    }

    query = format!("{} order by name, id desc", query);

    let mut current_page = 0;
    let mut limit = 0;
    let mut page_counts = 0;
    let params_slice: Vec<&(dyn ToSql + Sync)> = params.iter().map(AsRef::as_ref).collect();
    let row = client.query_one(&count_sql, &params_slice).await?;
    let total: i64 = row.get("total");
    if page.is_some() && per_page.is_some() {
        current_page = page.unwrap();
        limit = per_page.unwrap();
        let offset = (current_page - 1) * limit;
        query = format!("{query} limit {limit} offset {offset}");
        page_counts = (total as f64 / f64::from(limit)).ceil() as usize;
    }

    let items: Vec<Item> = client
        .query(&query, &params_slice[..])
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

    Ok(GetItemsResult {
        items,
        total,
        page: current_page,
        per_page: limit,
        page_counts,
    })
}
