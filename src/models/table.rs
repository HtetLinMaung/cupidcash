use actix_web::web;
use std::sync::Arc;
use tokio_postgres::Client;

#[derive(serde::Serialize)]
pub struct Table {
    pub id: i32,
    pub table_number: i32,
    pub qr_code: String,
}

pub async fn get_tables(
    shop_id: i32,
    client: &web::Data<Arc<Client>>,
) -> Result<Vec<Table>, tokio_postgres::Error> {
    let rows = client
        .query(
            "select id, table_number, qr_code from tables where deleted_at is null and shop_id = $1 order by table_number",
            &[&shop_id],
        )
        .await?;

    Ok(rows
        .into_iter()
        .map(|row| Table {
            id: row.get("id"),
            table_number: row.get("table_number"),
            qr_code: row.get("qr_code"),
        })
        .collect())
}
