// use std::time::SystemTime;

use chrono::{NaiveDate, NaiveDateTime};
// use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use tokio_postgres::{types::ToSql, Client, Error};

use crate::utils::{
    common_struct::PaginationResult,
    sql::{generate_pagination_query, PaginationOptions},
};

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
) -> Result<i32, Box<dyn std::error::Error>> {
    // Here, implement logic to insert the order into the database.
    // This might involve multiple insert statements: one for the order and then multiple for the items in the order.

    // Sample (you'd need to adapt this to your schema and logic)
    let row = client
        .query_one(
            "insert into orders (table_id, waiter_id) values ($1, $2) returning id",
            &[&order.table_id, &waiter_id],
        )
        .await?;
    let id: i32 = row.get("id");

    for item in order.items {
        client.execute(
            "insert into order_items (order_id, item_id, quantity, price, special_instructions) VALUES (currval(pg_get_serial_sequence('orders', 'id')), $1, $2,(select coalesce(price, 0.0) from items where id= $3 and deleted_at is null), $4)",
            &[&item.item_id, &item.quantity, &item.item_id,&item.special_instructions]
        ).await?;
    }

    Ok(id)
}

#[derive(Serialize)]
pub struct Order {
    pub id: i32,
    pub waiter_name: String,
    pub table_number: String,
    pub status: String,
    pub sub_total: f64,
    pub tax: f64,
    pub discount: f64,
    pub total: f64,
    pub shop_name: String,
    pub item_count: i64,
    pub created_at: NaiveDateTime,
}

pub async fn get_orders(
    search: &Option<String>,
    page: Option<usize>,
    per_page: Option<usize>,
    shop_id: i32,
    user_id: i32,
    role: &str,
    from_date: &Option<NaiveDate>,
    to_date: &Option<NaiveDate>,
    status: &Option<String>,
    client: &Client,
) -> Result<PaginationResult<Order>, Error> {
    let mut params: Vec<Box<dyn ToSql + Sync>> = vec![];
    let mut base_query = "from orders o inner join users u on u.id = o.waiter_id inner join tables t on o.table_id = t.id left join shops s on s.id = u.shop_id where u.deleted_at is null and o.deleted_at is null and s.deleted_at is null and t.deleted_at is null".to_string();
    if role == "Manager" {
        params.push(Box::new(shop_id));
        base_query = format!("{base_query} and t.shop_id = ${}", params.len());
    } else if role == "Waiter" {
        params.push(Box::new(shop_id));
        params.push(Box::new(user_id));
        base_query = format!(
            "{base_query} and t.shop_id = ${} and o.waiter_id = ${}",
            params.len() - 1,
            params.len()
        );
    }
    if from_date.is_some() && to_date.is_some() {
        params.push(Box::new(from_date.unwrap()));
        params.push(Box::new(to_date.unwrap()));
        base_query = format!(
            "{base_query} and o.created_at::date between ${} and ${}",
            params.len() - 1,
            params.len()
        );
    }

    if let Some(s) = status {
        params.push(Box::new(s));
        base_query = format!("{base_query} and o.status = ${}", params.len());
    }

    let sub_total_query = "(select sum(price * quantity) from order_items where order_id = o.id)";
    let select_columns = format!("o.id, u.name as waiter_name, t.table_number, o.status, o.tax::text, o.discount::text, coalesce({sub_total_query}, 0.0)::text as sub_total, coalesce({sub_total_query} - o.discount + o.tax, 0.0)::text as total, coalesce(s.name, '') shop_name, (select count(*) from order_items where order_id = o.id) as item_count, o.created_at");
    let order_options = "o.created_at desc";
    let result = generate_pagination_query(PaginationOptions {
        select_columns: &select_columns,
        base_query: &base_query,
        search_columns: vec![
            "u.name",
            "t.table_number",
            "o.status",
            "o.id::varchar",
            "s.name",
        ],
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

    let orders: Vec<Order> = client
        .query(&result.query, &params_slice)
        .await?
        .iter()
        .map(|row| {
            let sub_total: &str = row.get("sub_total");
            let tax: &str = row.get("tax");
            let discount: &str = row.get("discount");
            let total: &str = row.get("total");

            return Order {
                id: row.get("id"),
                waiter_name: row.get("waiter_name"),
                table_number: row.get("table_number"),
                status: row.get("status"),
                sub_total: sub_total.parse().unwrap(),
                tax: tax.parse().unwrap(),
                discount: discount.parse().unwrap(),
                total: total.parse().unwrap(),
                shop_name: row.get("shop_name"),
                item_count: row.get("item_count"),
                created_at: row.get("created_at"),
            };
        })
        .collect();

    Ok(PaginationResult {
        data: orders,
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
    table_number: String,
    created_at: NaiveDateTime,
    items: Vec<OrderItem>,
    status: String,
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
    role: &str,
    client: &Client,
) -> Result<OrderDetail, Error> {
    let mut params: Vec<Box<dyn ToSql + Sync>> = vec![Box::new(order_id)];
    let mut query = format!("select o.id, u.name as waiter_name, t.table_number, o.created_at, o.status from orders o inner join users u on u.id = o.waiter_id inner join tables t on o.table_id = t.id where u.deleted_at is null and o.deleted_at is null and t.deleted_at is null and o.id = $1");

    if role == "Waiter" {
        params.push(Box::new(shop_id));
        params.push(Box::new(user_id));
        query = format!(
            "{query} and t.shop_id = ${} and o.waiter_id = ${}",
            params.len() - 1,
            params.len()
        );
    } else if role == "Manager" {
        params.push(Box::new(shop_id));
        query = format!("{query} and t.shop_id = ${}", params.len());
    }

    let params_slice: Vec<&(dyn ToSql + Sync)> = params.iter().map(AsRef::as_ref).collect();
    let order_row = client.query_one(&query, &params_slice).await?;

    // Assume there's another table called order_items linking orders to items.
    let item_rows = client
        .query(
            "SELECT i.name as item_name, i.description, oi.price::text, i.image_url, oi.quantity, oi.special_instructions FROM order_items oi inner join items i on oi.item_id = i.id WHERE order_id = $1 and i.deleted_at is null order by i.name",
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
        status: order_row.get("status"),
        items,
    })
}

pub async fn get_order_by_id(
    order_id: i32,
    user_id: i32,
    shop_id: i32,
    role: &str,
    client: &Client,
) -> Option<Order> {
    let mut params: Vec<Box<dyn ToSql + Sync>> = vec![Box::new(order_id)];
    let sub_total_query = "(select sum(price * quantity) from order_items where order_id = o.id)";
    let mut base_query = format!("select o.id, u.name as waiter_name, t.table_number, o.status, o.tax::text, o.discount::text, coalesce({sub_total_query}, 0.0)::text as sub_total, coalesce({sub_total_query} - o.discount + o.tax, 0.0)::text as total, coalesce(s.name, '') shop_name, (select count(*) from order_items where order_id = o.id) as item_count, o.created_at from orders o inner join users u on u.id = o.waiter_id inner join tables t on o.table_id = t.id left join shops s on s.id = u.shop_id where u.deleted_at is null and o.deleted_at is null and t.deleted_at is null and o.id = $1");
    if role == "Manager" {
        params.push(Box::new(shop_id));
        base_query = format!("{base_query} and t.shop_id = ${}", params.len());
    } else if role == "Waiter" {
        params.push(Box::new(shop_id));
        params.push(Box::new(user_id));
        base_query = format!(
            "{base_query} and t.shop_id = ${} and o.waiter_id = ${}",
            params.len() - 1,
            params.len()
        );
    }
    let params_slice: Vec<&(dyn ToSql + Sync)> = params.iter().map(AsRef::as_ref).collect();
    match client.query_one(&base_query, &params_slice).await {
        Ok(row) => {
            let sub_total: &str = row.get("sub_total");
            let tax: &str = row.get("tax");
            let discount: &str = row.get("discount");
            let total: &str = row.get("total");
            Some(Order {
                id: row.get("id"),
                waiter_name: row.get("waiter_name"),
                table_number: row.get("table_number"),
                status: row.get("status"),
                sub_total: sub_total.parse().unwrap(),
                tax: tax.parse().unwrap(),
                discount: discount.parse().unwrap(),
                total: total.parse().unwrap(),
                shop_name: row.get("shop_name"),
                item_count: row.get("item_count"),
                created_at: row.get("created_at"),
            })
        }
        Err(err) => {
            println!("{:?}", err);
            None
        }
    }
}

pub async fn update_order(
    order_id: i32,
    status: &str,
    tax: f64,
    discount: f64,
    total: f64,
    client: &Client,
) -> Result<(), Box<dyn std::error::Error>> {
    let query = format!(
        "update orders set status = $1, tax = {}, discount = {}, total= {} where id = $2",
        tax, discount, total
    );
    client.execute(&query, &[&status, &order_id]).await?;
    Ok(())
}

pub async fn order_exists_in_table(table_id: &i32, client: &Client) -> Result<bool, Error> {
    // Execute a query to check if the order is not completed or canceled exists in the request table
    let row = client
        .query_one(
            "SELECT id FROM orders WHERE table_id = $1 and status in ('Pending','Served') ORDER BY id LIMIT 1",
            &[&table_id],
        )
        .await;

    // Return whether the user exists
    Ok(row.is_ok())
}
