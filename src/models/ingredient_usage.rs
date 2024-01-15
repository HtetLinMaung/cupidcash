use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use tokio_postgres::{types::ToSql, Client, Error};

use crate::utils::{
    common_struct::PaginationResult,
    sql::{generate_pagination_query, PaginationOptions},
};

#[derive(Debug, Serialize, Deserialize)]
pub struct IngredientUsage {
    pub usage_id: i32,
    pub ingredient_id: i32,
    pub quantity_used: f64,
    pub unit: String,
    pub usage_date: NaiveDateTime,
    pub associated_activity: Option<String>,
    pub notes: Option<String>,
    pub shop_id: Option<i32>,
    pub created_at: NaiveDateTime,
}

pub async fn get_ingredient_usages(
    search: &Option<String>,
    page: Option<usize>,
    per_page: Option<usize>,
    client: &Client,
) -> Result<PaginationResult<IngredientUsage>, Error> {
    let base_query =
        "from ingredient_usages iu join ingredients i on iu.ingredient_id = i.ingredient_id where iu.deleted_at is null".to_string();
    let params: Vec<Box<dyn ToSql + Sync>> = vec![];

    let order_options = "iu.created_at desc";

    let result = generate_pagination_query(PaginationOptions {
        select_columns: "iu.usage_id, iu.ingredient_id, iu.quantity_used::varchar, iu.unit, iu.usage_date, iu.associated_activity, iu.notes, iu.shop_id, iu.created_at",
        base_query: &base_query,
        search_columns: vec!["iu.usage_id::varchar", "iu.ingredient_id::varchar", "iu.usage_date", "iu.associated_activity", "iu.notes"],
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
    let mut ingredient_usages: Vec<IngredientUsage> = vec![];
    for row in &rows {
        let quantity_used_str: &str = row.get("quantity_used");
        ingredient_usages.push(IngredientUsage {
            usage_id: row.get("usage_id"),
            ingredient_id: row.get("ingredient_id"),
            quantity_used: quantity_used_str.parse().unwrap(),
            unit: row.get("unit"),
            usage_date: row.get("usage_date"),
            associated_activity: row.get("associated_activity"),
            notes: row.get("notes"),
            shop_id: row.get("shop_id"),
            created_at: row.get("created_at"),
        });
    }
    Ok(PaginationResult {
        data: ingredient_usages,
        total,
        page: current_page,
        per_page: limit,
        page_counts,
    })
}

#[derive(Deserialize)]
pub struct IngredientUsagesRequest {
    pub ingredient_usages: Vec<IngredientUsageRequest>,
    pub shop_id: Option<i32>,
}

#[derive(Deserialize)]
pub struct IngredientUsageRequest {
    pub usage_id: Option<i32>,
    pub ingredient_id: Option<i32>,
    pub quantity_used: Option<f64>,
    pub usage_date: String,
    pub associated_activity: String,
    pub notes: String,
}

pub async fn add_ingredient_usages(
    data: &IngredientUsagesRequest,
    client: &mut Client,
) -> Result<bool, Error> {
    let transaction = client.transaction().await?;
    for iur in &data.ingredient_usages {
        let row= transaction
            .query_one(
                "select stock_quantity::text from ingredients where ingredient_id = $1 and deleted_at is null for update",
                &[&iur.ingredient_id],
            )
            .await?;
        let remaining_quantity: &str = row.get("stock_quantity");
        let remaining_quantity: f64 = remaining_quantity.parse().unwrap();
        if iur.quantity_used.unwrap() > remaining_quantity {
            transaction.rollback().await?;
            return Ok(false);
        }

        transaction
            .execute(
                &format!(
                    "update ingredients set stock_quantity = stock_quantity - {} where ingredient_id = $1 and deleted_at is null",
                    iur.quantity_used.unwrap()
                ),
                &[&iur.ingredient_id],
            )
            .await?;

        let sql = format!(
            "insert into ingredient_usages (ingredient_id, quantity_used, unit, usage_date, associated_activity, notes, shop_id) values ($1, {}, (select unit from ingredients where ingredient_id = $2), '{}', $3, $4, $5)",
            iur.quantity_used.unwrap(), iur.usage_date
        );
        transaction
            .execute(
                &sql,
                &[
                    &iur.ingredient_id,
                    &iur.ingredient_id,
                    &iur.associated_activity,
                    &iur.notes,
                    &data.shop_id,
                ],
            )
            .await?;
    }
    transaction.commit().await?;
    Ok(true)
}

pub async fn get_ingredient_usage_by_id(usage_id: i32, client: &Client) -> Option<IngredientUsage> {
    let result = client
        .query_one(
            "SELECT usage_id, ingredient_id, quantity_used::varchar, unit, usage_date, associated_activity,
            notes, shop_id, created_at FROM ingredient_usages WHERE usage_id = $1 and deleted_at is null",
            &[&usage_id],
        )
        .await;

    match result {
        Ok(row) => {
            let quantity_used_str: &str = row.get("quantity_used");
            Some(IngredientUsage {
                usage_id: row.get("usage_id"),
                ingredient_id: row.get("ingredient_id"),
                quantity_used: quantity_used_str.parse().unwrap(),
                unit: row.get("unit"),
                usage_date: row.get("usage_date"),
                associated_activity: row.get("associated_activity"),
                notes: row.get("notes"),
                shop_id: row.get("shop_id"),
                created_at: row.get("created_at"),
            })
        }
        Err(_) => None,
    }
}

pub async fn update_ingredient_usage(
    data: &IngredientUsagesRequest,
    client: &mut Client,
) -> Result<bool, Box<dyn std::error::Error>> {
    let transaction = client.transaction().await?;
    for iur in &data.ingredient_usages {
        let row = transaction
            .query_one(
                "select quantity_used::varchar from ingredient_usages where usage_id = $1 and deleted_at is null for update",
                &[&iur.usage_id],
            )
            .await?;
        let used_quantity: &str = row.get("quantity_used");
        let used_quantity: f64 = used_quantity.parse().unwrap();
        let row = transaction
            .query_one(
                "select stock_quantity::varchar from ingredients where ingredient_id = $1 and deleted_at is null for update",
                &[&iur.ingredient_id],
            )
            .await?;
        let remaining_quantity: &str = row.get("stock_quantity");
        let remaining_quantity: f64 = remaining_quantity.parse().unwrap();
        let remaining_quantity = remaining_quantity + used_quantity;
        if iur.quantity_used.unwrap() > remaining_quantity {
            transaction.rollback().await?;
            return Ok(false);
        }

        transaction
                .execute(
                    &format!(
                        "update ingredients set stock_quantity = stock_quantity + {} - {} where ingredient_id = $1 and deleted_at is null",
                        used_quantity, iur.quantity_used.unwrap()
                    ),
                    &[&iur.ingredient_id],
                )
                .await?;

        let sql = format!(
                "update ingredient_usages set quantity_used = {}, usage_date = '{}', associated_activity = $1, notes = $2 where usage_id = $3",
                iur.quantity_used.unwrap(), iur.usage_date
            );
        transaction
            .execute(&sql, &[&iur.associated_activity, &iur.notes, &iur.usage_id])
            .await?;
    }
    transaction.commit().await?;
    Ok(true)
}

pub async fn delete_ingredient_usage(
    usage_id: i32,
    ingredient_id: i32,
    quantity_used: f64,
    client: &Client,
) -> Result<(), Box<dyn std::error::Error>> {
    client
    .execute(
        &format!(
            "update ingredients set stock_quantity = stock_quantity + {} where ingredient_id = $1 and deleted_at is null",
            quantity_used
        ),
        &[&ingredient_id],
    )
    .await?;
    client
        .execute(
            "update ingredient_usages set deleted_at = CURRENT_TIMESTAMP where usage_id = $1 and deleted_at is null",
            &[&usage_id],
        )
        .await?;

    Ok(())
}
