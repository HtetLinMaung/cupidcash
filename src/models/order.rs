// use std::time::SystemTime;

use chrono::{NaiveDate, NaiveDateTime};
// use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use std::env;
use tokio_postgres::{types::ToSql, Client, Error};

use futures::future::join_all;
use simple_pdf_generator::{Asset, AssetType, PrintOptions};
use simple_pdf_generator_derive::PdfTemplate;
use tokio::task::JoinError;

use crate::utils::{
    common_struct::PaginationResult,
    sql::{generate_pagination_query, PaginationOptions},
};

#[derive(Deserialize, Debug)]
pub struct NewOrder {
    pub table_id: i32,
    pub items: Vec<NewOrderItem>,
}

#[derive(Debug)]
struct ItemData {
    price: String,
    original_price: f64,
}

// Function to retrieve item data from the database
async fn get_item_data(
     client: &tokio_postgres::Transaction<'_>,
    item_id: i32,
) -> Result<ItemData, tokio_postgres::Error> {
    client
        .query_one(
            "SELECT 
                CASE
                    WHEN discount_type = 'No Discount' THEN price::text
                    WHEN discount_type = 'Discount by Specific Amount' THEN discounted_price::text
                    ELSE
                        CASE
                            WHEN discount_expiration IS NULL THEN (price - (price * discount_percent / 100))::text
                            WHEN NOW() >= discount_expiration THEN price::text
                            ELSE (price - (price * discount_percent / 100))::text
                        END
                END AS price,
                COALESCE(price, '0.0')::float8 AS original_price
            FROM items
            WHERE id = $1 AND deleted_at IS NULL",
            &[&item_id],
        )
        .await
        .map(|row| ItemData {
            price: row.get("price"),
            original_price: row.get("original_price"),
        })
}

// Function to insert order items
async fn insert_order_item(
    transaction: &tokio_postgres::Transaction<'_>,
    order_id: i32,
    item_id: i32,
    quantity: i32,
    special_instructions: &str,
    price: f64,
    original_price: f64,
) -> Result<(), tokio_postgres::Error> {
    let query = format!("INSERT INTO order_items (order_id, item_id, quantity, special_instructions, price, original_price) 
    VALUES ($1, $2, $3, $4, {},{})", &price, &original_price);
    transaction
        .execute(
            &query,
            &[&order_id, &item_id, &quantity, &special_instructions],
        )
        .await
        .map(|_| ())
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
    client: &mut Client,
) -> Result<i32, Box<dyn std::error::Error>> {
    let transaction = client.transaction().await?;

    // Here, implement logic to insert the order into the database.
    // This might involve multiple insert statements: one for the order and then multiple for the items in the order.

    // Sample (you'd need to adapt this to your schema and logic)
    for item in &order.items {
        let stock_row = transaction
            .query_one(
                "SELECT stock_quantity,name FROM items WHERE id = $1 AND deleted_at IS NULL FOR UPDATE",
                &[&item.item_id],
            )
            .await?;
        let remaining_quantity: i32 = stock_row.get("stock_quantity");
        if item.quantity > remaining_quantity {
            let item_name: String = stock_row.get("name");
            transaction.rollback().await?;
            return Err(format!("Insufficient stock for item {}: requested {}, remaining {}", item_name, item.quantity, remaining_quantity).into());

        }
    }


    let row = transaction
        .query_one(
            "insert into orders (table_id, waiter_id) values ($1, $2) returning id",
            &[&order.table_id, &waiter_id],
        )
        .await?;
    let id: i32 = row.get("id");
    for item in order.items {
        // Retrieve item data from the database
        let item_data = get_item_data(&transaction, item.item_id).await?;

        // Insert order item using retrieved data
        transaction
        .execute(
            "UPDATE items SET stock_quantity = stock_quantity - $1 WHERE id = $2 AND deleted_at IS NULL",
            &[&item.quantity, &item.item_id],
        )
        .await?;

        insert_order_item(
            &transaction,
            id,
            item.item_id,
            item.quantity,
            &item.special_instructions,
            item_data.price.parse().unwrap_or_default(),
            item_data.original_price,
        )
        .await?;
    }
    transaction.commit().await?;
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
    original_price: f64,
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
            "SELECT i.name as item_name, i.description, oi.price::text, oi.original_price::text, i.image_url, oi.quantity, oi.special_instructions FROM order_items oi inner join items i on oi.item_id = i.id WHERE order_id = $1 and i.deleted_at is null order by i.name",
            &[&order_id],
        )
        .await?;

    let items: Vec<OrderItem> = item_rows
        .iter()
        .map(|row| {
            let price: &str = row.get("price");
            let price: f64 = price.parse().unwrap();
            let original_price: &str = row.get("original_price");
            let original_price: f64 = original_price.parse().unwrap();
            OrderItem {
                item_name: row.get("item_name"),
                description: row.get("description"),
                price,
                original_price,
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
    client: &Client,
) -> Result<(), Box<dyn std::error::Error>> {
    let query = format!(
        "update orders set status = $1, tax = {}, discount = {} where id = $2",
        tax, discount
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

#[derive(Serialize)]
pub struct DailySaleReportData {
    item_id: i32,
    item_name: String,
    quantity: i32,
    amount: f64,
    discount: f64,
    netsale: f64,
}

#[derive(PdfTemplate, Serialize)]
pub struct DailySaleReportSummaryData {
    total_quantity: i32,
    total_amount: f64,
    total_discount: f64,
    total_netsale: f64,
    #[PdfTableData]
    data_list: Vec<DailySaleReportData>,
}

pub async fn get_daily_sale_report(
    date: NaiveDate,
    shop_id: i32,
    client: &tokio_postgres::Client,
) -> Result<DailySaleReportSummaryData, tokio_postgres::Error> {
    let query = format!(
        "select oi.item_id, i.name, sum(oi.quantity)::text as quantity,
    (sum(oi.original_price*oi.quantity))::text as amount,
    (sum(oi.original_price*oi.quantity)-sum(oi.price*oi.quantity))::text as discount,
    (sum(oi.price*oi.quantity))::text as netsale,
    sum(oi.price*oi.quantity) as netsaleorder
    from orders o, items i, order_items oi, tables t
    where o.id = oi.order_id 
    and i.id = oi.item_id
    and o.table_id=t.id
    and DATE_TRUNC('day', o.created_at)='{}'
    and t.shop_id=$1
    group by oi.item_id, i.name
    order by netsaleorder desc",
        &date
    );
    let mut total_amount: f64 = 0.0;
    let mut total_discount: f64 = 0.0;
    let mut total_netsale: f64 = 0.0;
    let mut total_quantity: i32 = 0;
    let item_rows = client.query(&query, &[&shop_id]).await?;
    let data_list: Vec<DailySaleReportData> = item_rows
        .iter()
        .map(|row| {
            let amount: &str = row.get("amount");
            let amount: f64 = amount.parse().unwrap();
            let discount: &str = row.get("discount");
            let discount: f64 = discount.parse().unwrap();
            let netsale: &str = row.get("netsale");
            let netsale: f64 = netsale.parse().unwrap();
            let quantity: &str = row.get("quantity");
            let quantity: i32 = quantity.parse().unwrap();
            total_amount += amount;
            total_discount += discount;
            total_netsale += netsale;
            total_quantity += quantity;
            DailySaleReportData {
                item_id: row.get("item_id"),
                item_name: row.get("name"),
                amount,
                discount,
                quantity,
                netsale,
            }
        })
        .collect();
    let data: DailySaleReportSummaryData = DailySaleReportSummaryData {
        total_amount,
        total_discount,
        total_netsale,
        total_quantity,
        data_list,
    };
    if let Err(err) = prepare_daily_sale_report_pdf(&data).await {
        eprintln!("Error: {}", err);
    }
    Ok(data)
}

async fn prepare_daily_sale_report_pdf(data: &DailySaleReportSummaryData) -> Result<(), JoinError> {
    let html_path = env::current_dir()
        .unwrap()
        // .join("test_suite")
        .join("src/template/daily-sale-report.html");

    let assets = [Asset {
        path: env::current_dir()
            .unwrap()
            //.join("test_suite")
            .join("src/template/css/style.css"),
        r#type: AssetType::Style,
    }];

    let print_options = PrintOptions {
        paper_width: Some(210.0),
        paper_height: Some(297.0),
        margin_top: Some(10.0),
        margin_bottom: Some(10.0),
        margin_left: Some(10.0),
        margin_right: Some(10.0),
        ..PrintOptions::default()
    };
    let gen_0 = data.generate_pdf(html_path.clone(), &assets, &print_options);

    let futures_res = join_all(vec![gen_0]).await;

    for res in futures_res.iter().enumerate() {
        let Ok(content) = res.1.as_ref() else {
            println!("Error on {} {}", res.0, res.1.as_ref().unwrap_err());
            continue;
        };

        _ = tokio::fs::write(format!("reports/dailysalereport.pdf"), content).await;
    }
    Ok(())
}
