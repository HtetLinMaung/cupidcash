use std::sync::Arc;

use actix_web::web;
use serde::{Deserialize, Serialize};
use tokio_postgres::Client;

#[derive(Serialize, Deserialize, Debug)]
pub struct Category {
    pub id: i32,
    pub name: String,
    pub description: String,
}

pub async fn get_categories(
    shop_id: i32,
    client: &web::Data<Arc<Client>>,
) -> Result<Vec<Category>, tokio_postgres::Error> {
    let rows = client
        .query(
            "select id, name, description from categories where deleted_at is null and shop_id = $1 order by name, id desc",
            &[&shop_id],
        )
        .await?;

    Ok(rows
        .into_iter()
        .map(|row| Category {
            id: row.get(0),
            name: row.get(1),
            description: row.get(2),
        })
        .collect())
}
