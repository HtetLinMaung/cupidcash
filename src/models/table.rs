use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use tokio_postgres::{types::ToSql, Client, Error};

use crate::utils::{
    common_struct::PaginationResult,
    sql::{generate_pagination_query, PaginationOptions},
};

#[derive(Debug, Serialize, Deserialize)]
pub struct Table {
    pub id: i32,
    pub table_number: String,
    pub qr_code: String,
    pub shop_id: i32,
    pub shop_name: String,
    pub created_at: NaiveDateTime,
}

pub async fn get_tables(
    search: &Option<String>,
    page: Option<usize>,
    per_page: Option<usize>,
    role: &str,
    shop_id: i32,
    client: &Client,
) -> Result<PaginationResult<Table>, Error> {
    let mut base_query =
        "from tables t join shops s on s.id = t.shop_id where t.deleted_at is null".to_string();
    let mut params: Vec<Box<dyn ToSql + Sync>> = vec![];

    let order_options = if role == "Waiter" {
        "table_number"
    } else {
        "t.created_at desc"
    };

    if role == "Waiter" {
        params.push(Box::new(shop_id));
        base_query = format!("{base_query} and t.shop_id = ${}", params.len());
    }

    let result = generate_pagination_query(PaginationOptions {
        select_columns:
            "t.id, t.table_number, t.qr_code, t.shop_id, s.name shop_name, t.created_at",
        base_query: &base_query,
        search_columns: vec!["t.id::varchar", "t.table_number", "t.qr_code", "s.name"],
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

    let tables: Vec<Table> = client
        .query(&result.query, &params_slice)
        .await?
        .iter()
        .map(|row| Table {
            id: row.get("id"),
            table_number: row.get("table_number"),
            qr_code: row.get("qr_code"),
            shop_id: row.get("shop_id"),
            shop_name: row.get("shop_name"),
            created_at: row.get("created_at"),
        })
        .collect();

    Ok(PaginationResult {
        data: tables,
        total,
        page: current_page,
        per_page: limit,
        page_counts,
    })
}

#[derive(Debug, Deserialize)]
pub struct TableRequest {
    pub table_number: String,
    pub qr_code: String,
    pub shop_id: i32,
}

pub async fn add_table(
    data: &TableRequest,
    client: &Client,
) -> Result<(), Box<dyn std::error::Error>> {
    client
        .execute(
            "insert into tables (table_number, qr_code, shop_id) values ($1, $2, $3)",
            &[&data.table_number, &data.qr_code, &data.shop_id],
        )
        .await?;
    Ok(())
}

pub async fn get_table_by_id(table_id: i32, client: &Client) -> Option<Table> {
    let result = client
        .query_one(
            "select t.id, t.table_number, t.qr_code, s.id shop_id, s.name shop_name, t.created_at from tables t join shops s on s.id = t.shop_id where t.deleted_at is null and t.id = $1",
            &[&table_id],
        )
        .await;

    match result {
        Ok(row) => Some(Table {
            id: row.get("id"),
            table_number: row.get("table_number"),
            qr_code: row.get("qr_code"),
            shop_id: row.get("shop_id"),
            shop_name: row.get("shop_name"),
            created_at: row.get("created_at"),
        }),
        Err(_) => None,
    }
}

pub async fn update_table(
    table_id: i32,
    data: &TableRequest,
    client: &Client,
) -> Result<(), Box<dyn std::error::Error>> {
    client
        .execute(
            "update tables set table_number = $1, qr_code = $2, shop_id = $3 where id = $4",
            &[&data.table_number, &data.qr_code, &data.shop_id, &table_id],
        )
        .await?;

    Ok(())
}

pub async fn delete_table(
    table_id: i32,
    client: &Client,
) -> Result<(), Box<dyn std::error::Error>> {
    client
        .execute(
            "update tables set deleted_at = CURRENT_TIMESTAMP where id = $1",
            &[&table_id],
        )
        .await?;

    Ok(())
}

pub async fn table_number_exists(
    table_number: &str,
    shop_id: &i32,
    client: &Client,
) -> Result<bool, Error> {
    // Execute a query to check if the table_number exists in the tables table
    let row = client
        .query_one(
            "SELECT table_number FROM tables WHERE table_number = $1 and shop_id=$2 and deleted_at is null",
            &[&table_number, &shop_id],
        )
        .await;

    // Return whether the user exists
    Ok(row.is_ok())
}

//order_id with table data
#[derive(Debug, Serialize)]
pub struct DashboardData {
    pub id: i32,
    pub table_number: String,
    pub qr_code: String,
    pub shop_id: i32,
    pub shop_name: String,
    pub created_at: NaiveDateTime,
    pub order_id: i32,
}
//getdashboarddata
//left join orders (status not 'Canceled','Completed') with tables
pub async fn getdashboarddata(
    search: &Option<String>,
    page: Option<usize>,
    per_page: Option<usize>,
    role: &str,
    shop_id: i32,
    client: &Client,
) -> Result<PaginationResult<DashboardData>, Error> {
    let mut base_query =
        "from tables t join shops s on s.id = t.shop_id 
        left join orders o on t.id=o.table_id and o.status not in ('Canceled','Completed')
        where t.deleted_at is null"
            .to_string();
    let mut params: Vec<Box<dyn ToSql + Sync>> = vec![];

    let order_options = if role == "Waiter" {
        "table_number"
    } else {
        "t.created_at desc"
    };

    if role == "Waiter" {
        params.push(Box::new(shop_id));
        base_query = format!("{base_query} and t.shop_id = ${}", params.len());
    }

    let result = generate_pagination_query(PaginationOptions {
        select_columns:
            "t.id, t.table_number, t.qr_code, t.shop_id, s.name shop_name, t.created_at, COALESCE(o.id, 0) order_id",
        base_query: &base_query,
        search_columns: vec!["t.id::varchar", "t.table_number", "t.qr_code", "s.name"],
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

    let dashboard_data: Vec<DashboardData> = client
        .query(&result.query, &params_slice)
        .await?
        .iter()
        .map(|row| DashboardData {
            id: row.get("id"),
            table_number: row.get("table_number"),
            qr_code: row.get("qr_code"),
            shop_id: row.get("shop_id"),
            shop_name: row.get("shop_name"),
            created_at: row.get("created_at"),
            order_id: row.get("order_id")
        })
        .collect();

    Ok(PaginationResult {
        data: dashboard_data,
        total,
        page: current_page,
        per_page: limit,
        page_counts,
    })
}
