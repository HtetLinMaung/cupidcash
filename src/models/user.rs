use bcrypt::{hash, DEFAULT_COST};
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use tokio_postgres::{types::ToSql, Client, Error};

use crate::utils::{
    common_struct::PaginationResult,
    sql::{generate_pagination_query, PaginationOptions},
};

#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    pub id: i32,
    pub name: String,
    pub username: String,
    pub password: String,
    pub role_id: i32,
    pub role_name: String,
    pub shop_id: i32,
    pub shop_name: String,
    pub created_at: NaiveDateTime,
}

pub async fn get_user(username: &str, client: &Client) -> Option<User> {
    let result = client
        .query_one(
            "select u.id, u.username, u.password, u.role_id, r.role_name, u.name, coalesce(u.shop_id, 0) as shop_id, coalesce(s.name, '') as shop_name, u.created_at from users u inner join roles r on r.id = u.role_id left join shops s on s.id = u.shop_id where u.username = $1 and u.deleted_at is null and r.deleted_at is null and s.deleted_at is null",
            &[&username],
        )
        .await;

    match result {
        Ok(row) => Some(User {
            id: row.get("id"),
            name: row.get("name"),
            username: row.get("username"),
            password: row.get("password"),
            role_id: row.get("role_id"),
            role_name: row.get("role_name"),
            shop_id: row.get("shop_id"),
            shop_name: row.get("shop_name"),
            created_at: row.get("created_at"),
        }),
        Err(_) => None,
    }
}

#[derive(Deserialize)]
pub struct AddUserRequest {
    pub name: String,
    pub username: String,
    pub password: String,
    pub role_id: i32,
    pub shop_id: i32,
}

pub async fn add_user(
    data: &AddUserRequest,
    client: &Client,
) -> Result<(), Box<dyn std::error::Error>> {
    let hashed_password = hash(&data.password, DEFAULT_COST)
        .map_err(|e| format!("Failed to hash password: {}", e))?;
    client.execute(
        "insert into users (name, username, password, role_id, shop_id) values ($1, $2, $3, $4, $5)",
        &[&data.name, &data.username, &hashed_password, &data.role_id, &data.shop_id],
    ).await?;
    Ok(())
}

pub async fn get_users(
    search: &Option<String>,
    page: Option<usize>,
    per_page: Option<usize>,
    role_id: Option<i32>,
    client: &Client,
) -> Result<PaginationResult<User>, Error> {
    let mut base_query =
        "from users u join roles r on u.role_id = r.id left join shops s on s.id = u.shop_id where u.deleted_at is null and r.deleted_at is null and s.deleted_at is null".to_string();
    let mut params: Vec<Box<dyn ToSql + Sync>> = vec![];

    if let Some(ri) = role_id {
        params.push(Box::new(ri));
        base_query = format!("{base_query} and u.role_id = ${}", params.len());
    }

    let result = generate_pagination_query(PaginationOptions {
        select_columns:
            "u.id, u.name, u.username, u.password, u.role_id, r.role_name, coalesce(u.shop_id, 0) as shop_id, coalesce(s.name, '') as shop_name, u.created_at",
        base_query: &base_query,
        search_columns: vec![
            "u.id::varchar",
            "u.name",
            "u.username",
            "r.role_name",
            "coalesce(s.name, '')",
        ],
        search: search.as_deref(),
        order_options: Some("u.created_at desc"),
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

    let users = client
        .query(&result.query, &params_slice[..])
        .await?
        .iter()
        .map(|row| User {
            id: row.get("id"),
            name: row.get("name"),
            username: row.get("username"),
            password: row.get("password"),
            role_id: row.get("role_id"),
            role_name: row.get("role_name"),
            shop_id: row.get("shop_id"),
            shop_name: row.get("shop_name"),
            created_at: row.get("created_at"),
        })
        .collect();

    Ok(PaginationResult {
        data: users,
        total,
        page: current_page,
        per_page: limit,
        page_counts,
    })
}

pub async fn get_user_by_id(user_id: i32, client: &Client) -> Option<User> {
    match client.query_one("select u.id, u.name, u.username, u.password, u.role_id, r.role_name, coalesce(u.shop_id, 0) as shop_id, coalesce(s.name, '') as shop_name, u.created_at from users u join roles r on u.role_id = r.id left join shops s on s.id = u.shop_id where u.deleted_at is null and r.deleted_at is null and s.deleted_at is null and u.id = $1", &[&user_id]).await {
        Ok(row) => Some(User {
            id: row.get("id"),
            name: row.get("name"),
            username: row.get("username"),
            password: row.get("password"),
            role_id: row.get("role_id"),
            role_name: row.get("role_name"),
            shop_id: row.get("shop_id"),
            shop_name: row.get("shop_name"),
            created_at: row.get("created_at"),
        }),
        Err(err) => {
            println!("{:?}", err);
            None
        }
    }
}

#[derive(Deserialize)]
pub struct UpdateUserRequest {
    pub name: String,
    pub password: String,
    pub role_id: i32,
    pub shop_id: i32,
}

pub async fn update_user(
    old_password: &str,
    data: &UpdateUserRequest,
    user_id: i32,
    client: &Client,
) -> Result<(), Box<dyn std::error::Error>> {
    let password: &str = &data.password;
    let mut hashed_password = password.to_string();

    if password != old_password {
        hashed_password = hash(&data.password.as_str(), DEFAULT_COST)
            .map_err(|e| format!("Failed to hash password: {}", e))?;
    }

    client
        .execute(
            "update users set name = $1, password = $2, role_id = $3, shop_id = $4 where id = $5",
            &[
                &data.name,
                &hashed_password,
                &data.role_id,
                &data.shop_id,
                &user_id,
            ],
        )
        .await?;
    Ok(())
}

pub async fn delete_user(user_id: i32, client: &Client) -> Result<(), Box<dyn std::error::Error>> {
    client
        .execute(
            "update users set deleted_at = CURRENT_TIMESTAMP where id = $1 and deleted_at is null",
            &[&user_id],
        )
        .await?;
    Ok(())
}

pub async fn user_exists(username: &str, client: &Client) -> Result<bool, Error> {
    // Execute a query to check if the username exists in the users table
    let row = client
        .query_one(
            "SELECT username FROM users WHERE username = $1 and deleted_at is null",
            &[&username],
        )
        .await;

    // Return whether the user exists
    Ok(row.is_ok())
}
