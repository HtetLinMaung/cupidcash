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

pub async fn get_items(
    shop_id: i32,
    name: &Option<String>,
    category_id: &Option<i32>,
    client: &Client,
) -> Result<Vec<Item>, Error> {
    let mut query =
        "select id, name, description, price::text, image_url from items where shop_id = $1 and deleted_at is null".to_string();
    let mut params: Vec<Box<dyn ToSql + Sync>> = vec![Box::new(shop_id)];
    if let Some(n) = name {
        query = format!("{} and name like '%{}%'", query, n);
    }

    if let Some(c) = category_id {
        query = format!("{} and category_id = $2", query);
        params.push(Box::new(c));
    }

    query = format!("{} order by name, id desc", query);

    let params_slice: Vec<&(dyn ToSql + Sync)> = params.iter().map(AsRef::as_ref).collect();

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

    Ok(items)
}
