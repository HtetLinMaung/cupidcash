use chrono::{NaiveDateTime, NaiveDate};
use serde::{Deserialize, Serialize};
use tokio_postgres::{types::ToSql, Client, Error};

use crate::utils::{
    common_struct::PaginationResult,
    sql::{generate_pagination_query, PaginationOptions},
};

#[derive(Debug, Serialize, Deserialize)]
pub struct Ingredient {
    pub ingredient_id: i32,
    pub name: String,
    pub stock_quantity: f32,
    pub unit: String,
    pub reorder_level: f32,
    pub expiry_date: NaiveDate,
    pub created_at: NaiveDateTime,

}

pub async fn get_ingredients(
    search: &Option<String>,
    page: Option<usize>,
    per_page: Option<usize>,
    client: &Client,
) -> Result<PaginationResult<Ingredient>, Error> {
    let base_query = "from ingredients where deleted_at is null".to_string();
    let params: Vec<Box<dyn ToSql + Sync>> = vec![];

    let order_options = "created_at desc";

    let result = generate_pagination_query(PaginationOptions {
        select_columns: "ingredient_id, name, stock_quantity::text as stock_quantity, unit, reorder_level::text as reorder_level, expiry_date::text as expiry_date, created_at",
        base_query: &base_query,
        search_columns: vec!["ingredient_id::varchar", "name","unit", "reorder_level::varchar","expiry_date::varchar"],
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

    let rows= client
        .query(&result.query, &params_slice)
        .await?;
    let mut ingredients: Vec<Ingredient> = vec![];
    for row in &rows {
        let stock_quantity_str: &str = row.get("stock_quantity");
        let reorder_level_str: &str = row.get("reorder_level");
        let expiry_date_str: &str = row.get("expiry_date");

        ingredients.push(
            Ingredient {
            ingredient_id: row.get("ingredient_id"),
            name: row.get("name"),
            stock_quantity: stock_quantity_str.parse().unwrap(),
            unit: row.get("unit"),
            reorder_level: reorder_level_str.parse().unwrap(),
            expiry_date: expiry_date_str.parse().unwrap(),
            created_at: row.get("created_at"),
            }
        );
    }

    Ok(PaginationResult {
        data: ingredients,
        total,
        page: current_page,
        per_page: limit,
        page_counts,
    })
}

#[derive(Debug, Deserialize)]
pub struct IngredientRequest {
    pub name: String,
    pub stock_quantity: f32,
    pub unit: String,
    pub reorder_level: f32,
    pub expiry_date: NaiveDate,
}

pub async fn add_ingredient(
    data: &IngredientRequest,
    client: &Client,
) -> Result<(), Box<dyn std::error::Error>> {
    let insert_query = format!("insert into ingredients (name, stock_quantity, unit, reorder_level, expiry_date ) values ($1,{}, $2, {}, $3)",data.stock_quantity,data.reorder_level);
    client
        .execute(&insert_query,
            &[&data.name, &data.unit, &data.expiry_date],
        )
        .await?;
    Ok(())
}

pub async fn get_ingredient_by_id(ingredient_id: i32, client: &Client) -> Option<Ingredient> {
    let result = client
        .query_one(
            "select ingredient_id, name, stock_quantity::text as stock_quantity, unit, reorder_level::text as reorder_level, expiry_date::text as expiry_date, created_at from ingredients where deleted_at is null and ingredient_id = $1",
            &[&ingredient_id],
        )
        .await;
   
    match result {
        Ok(row) =>{
            let stock_quantity_str: &str = row.get("stock_quantity");
            let reorder_level_str: &str = row.get("reorder_level");
            let expiry_date_str: &str = row.get("expiry_date");
            Some(Ingredient {
                ingredient_id: row.get("ingredient_id"),
                name: row.get("name"),
                stock_quantity: stock_quantity_str.parse().unwrap(),
                unit: row.get("unit"),
                reorder_level: reorder_level_str.parse().unwrap(),
                expiry_date: expiry_date_str.parse().unwrap(),
                created_at: row.get("created_at"),
            })
        },
        Err(_) => None,
    }
}

pub async fn update_ingredient(
    ingredient_id: i32,
    data: &IngredientRequest,
    client: &Client,
) -> Result<(), Box<dyn std::error::Error>> {
    let update_query = format!("update ingredients set name = $1, reorder_level = {}, expiry_date = $2 where ingredient_id = $3",data.reorder_level);
    client
        .execute(
            &update_query,
            &[&data.name, &data.expiry_date, &ingredient_id],
        )
        .await?;

    Ok(())
}

pub async fn delete_ingredient(
    ingredient_id: i32,
    client: &Client,
) -> Result<(), Box<dyn std::error::Error>> {
    client
        .execute(
            "update ingredients set deleted_at = CURRENT_TIMESTAMP where ingredient_id = $1",
            &[&ingredient_id],
        )
        .await?;

    Ok(())
}
