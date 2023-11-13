use serde::Serialize;
use tokio_postgres::Client;

#[derive(Serialize)]
pub struct Role {
    pub id: i32,
    pub role_name: String,
}

pub async fn get_roles(client: &Client) -> Result<Vec<Role>, Box<dyn std::error::Error>> {
    let rows = client
        .query(
            "select id, role_name from roles where deleted_at is null order by role_name",
            &[],
        )
        .await?;
    Ok(rows
        .iter()
        .map(|row| Role {
            id: row.get("id"),
            role_name: row.get("role_name"),
        })
        .collect())
}
