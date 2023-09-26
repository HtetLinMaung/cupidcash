// use std::time::SystemTime;

use chrono::{NaiveDate, NaiveDateTime};
// use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use tokio_postgres::{types::ToSql, Client, Error};

#[derive(Deserialize, Debug)]
pub struct NewOrder {
    pub table_id: i32,
    pub items: Vec<NewOrderItem>,
}

#[derive(Deserialize, Debug, Clone, Serialize)]
pub struct NewOrderItem {
    pub item_id: i32,
    pub quantity: i32,
    pub special_instructions: String,
}

pub async fn create_order(
    waiter_id: i32,
    order: NewOrder,
    client: &Client,
) -> Result<(), Box<dyn std::error::Error>> {
    // Here, implement logic to insert the order into the database.
    // This might involve multiple insert statements: one for the order and then multiple for the items in the order.

    // Sample (you'd need to adapt this to your schema and logic)
    client
        .execute(
            "insert into orders (table_id, waiter_id) values ($1, $2)",
            &[&order.table_id, &waiter_id],
        )
        .await?;

    for item in order.items {
        client.execute(
            "insert into order_items (order_id, item_id, quantity, special_instructions) VALUES (currval(pg_get_serial_sequence('orders', 'id')), $1, $2, $3)",
            &[&item.item_id, &item.quantity, &item.special_instructions]
        ).await?;
    }

    Ok(())
}

#[derive(Serialize)]
pub struct Order {
    id: i32,
    waiter_name: String,
    table_number: i32,
    created_at: NaiveDateTime,
}

pub struct GetOrdersResult {
    pub orders: Vec<Order>,
    pub total: i64,
    pub page: u32,
    pub per_page: u32,
    pub page_counts: usize,
}

pub async fn get_orders(
    shop_id: i32,
    user_id: i32,
    role: &str,
    page: &Option<u32>,
    per_page: &Option<u32>,
    from_date: &Option<NaiveDate>,
    to_date: &Option<NaiveDate>,
    client: &Client,
) -> Result<GetOrdersResult, Error> {
    let mut params: Vec<Box<dyn ToSql + Sync>> = vec![];
    let mut sql = "select o.id, u.name as waiter_name, t.table_number, o.created_at from orders o inner join users u on u.id = o.waiter_id inner join tables t on o.table_id = t.id where u.deleted_at is null and o.deleted_at is null and t.deleted_at is null".to_string();
    let mut count_sql = "select count(*) as total from orders o inner join users u on u.id = o.waiter_id inner join tables t on o.table_id = t.id where u.deleted_at is null and o.deleted_at is null and t.deleted_at is null".to_string();
    if role == "Manager" {
        params.push(Box::new(shop_id));
        sql = format!("{sql} and t.shop_id = $1");
        count_sql = format!("{count_sql} and t.shop_id = $1");
    } else if role == "Waiter" {
        params.push(Box::new(shop_id));
        params.push(Box::new(user_id));
        sql = format!("{sql} and t.shop_id = $1 and o.waiter_id = $2");
        count_sql = format!("{count_sql} and t.shop_id = $1 and o.waiter_id = $2");
    }
    if from_date.is_some() && to_date.is_some() {
        params.push(Box::new(from_date.unwrap()));
        params.push(Box::new(to_date.unwrap()));
        sql = format!(
            "{sql} and o.created_at::date between ${} and ${}",
            params.len() - 1,
            params.len()
        );
        count_sql = format!(
            "{count_sql} and o.created_at::date between ${} and ${}",
            params.len() - 1,
            params.len()
        );
    }
    sql = format!("{sql} order by o.created_at desc");

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
        sql = format!("{sql} limit {limit} offset {offset}");
        page_counts = (total as f64 / f64::from(limit)).ceil() as usize;
    }
    let rows = client.query(&sql, &params_slice).await?;

    Ok(GetOrdersResult {
        orders: rows
            .into_iter()
            .map(|row| Order {
                id: row.get("id"),
                waiter_name: row.get("waiter_name"),
                table_number: row.get("table_number"),
                created_at: row.get("created_at"),
            })
            .collect(),
        total,
        page: current_page,
        per_page: limit,
        page_counts,
    })
}

#[derive(Serialize)]
pub struct OrderDetail {
    id: i32,
    waiter_name: String,
    table_number: i32,
    created_at: NaiveDateTime,
    items: Vec<OrderItem>,
}

#[derive(Serialize)]
pub struct OrderItem {
    item_name: String,
    description: String,
    price: f64,
    image_url: String,
    quantity: i32,
    special_instructions: String,
}

pub async fn get_order_detail(
    shop_id: i32,
    user_id: i32,
    order_id: i32,
    client: &Client,
) -> Result<OrderDetail, Error> {
    let order_row = client
        .query_one("select o.id, u.name as waiter_name, t.table_number, o.created_at from orders o inner join users u on u.id = o.waiter_id inner join tables t on o.table_id = t.id where u.deleted_at is null and o.deleted_at is null and t.deleted_at is null and t.shop_id = $1 and o.waiter_id = $2 and o.id = $3", &[&shop_id, &user_id,&order_id])
        .await?;

    // Assume there's another table called order_items linking orders to items.
    let item_rows = client
        .query(
            "SELECT i.name as item_name, i.description, i.price::text, i.image_url, oi.quantity, oi.special_instructions FROM order_items oi inner join items i on oi.item_id = i.id WHERE order_id = $1 and i.deleted_at is null order by i.name",
            &[&order_id],
        )
        .await?;

    let items: Vec<OrderItem> = item_rows
        .iter()
        .map(|row| {
            let price: &str = row.get("price");
            let price: f64 = price.parse().unwrap();
            OrderItem {
                item_name: row.get("item_name"),
                description: row.get("description"),
                price,
                image_url: row.get("image_url"),
                quantity: row.get("quantity"),
                special_instructions: row.get("special_instructions"),
            }
        })
        .collect();

    Ok(OrderDetail {
        id: order_row.get("id"),
        waiter_name: order_row.get("waiter_name"),
        table_number: order_row.get("table_number"),
        created_at: order_row.get("created_at"),
        items,
    })
}
