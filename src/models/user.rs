use serde::{Deserialize, Serialize};
use tokio_postgres::Client;

#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    pub id: i32,
    pub username: String,
    pub password: String,
    pub role_name: String,
    pub name: String,
    pub shop_id: i32,
}

pub async fn get_user(username: &str, client: &Client) -> Option<User> {
    // Here we fetch the user from the database using tokio-postgres
    // In a real-world scenario, handle errors gracefully
    let result = client
        .query_one(
            "select u.id, u.username, u.password, r.role_name, u.name, coalesce(u.shop_id, 0) as shop_id from users u inner join roles r on r.id = u.role_id where username = $1 and u.deleted_at is null and r.deleted_at is null",
            &[&username],
        )
        .await;

    match result {
        Ok(row) => Some(User {
            id: row.get(0),
            username: row.get(1),
            password: row.get(2),
            role_name: row.get(3),
            name: row.get(4),
            shop_id: row.get(5),
        }),
        Err(_) => None,
    }
}
