use serde::Deserialize;
use tokio_postgres::{Client, Error};

#[derive(Deserialize)]
pub struct IngredientUsagesRequest {
    ingredient_usages: Vec<IngredientUsageRequest>,
}

#[derive(Deserialize)]
pub struct IngredientUsageRequest {
    pub ingredient_id: i32,
    pub quantity_used: f64,
    pub usage_date: String,
    pub associated_activity: String,
    pub notes: String,
}

pub async fn add_ingredient_usages(
    data: &IngredientUsagesRequest,
    shop_id: i32,
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
        if iur.quantity_used > remaining_quantity {
            transaction.rollback().await?;
            return Ok(false);
        }

        transaction
            .execute(
                &format!(
                    "update ingredients set stock_quantity = stock_quantity - ${} where ingredient_id = $1 and deleted_at is null",
                    iur.quantity_used
                ),
                &[&iur.ingredient_id],
            )
            .await?;

        let sql = format!(
            "insert into ingredient_usages (ingredient_id, quantity_used, unit, usage_date, associated_activity, notes, shop_id) values ($1, ${}, (select unit from ingredients where ingredient_id = $2), ${}, $3, $4, $5)",
            iur.quantity_used, iur.usage_date
        );
        transaction
            .execute(
                &sql,
                &[
                    &iur.ingredient_id,
                    &iur.ingredient_id,
                    &iur.associated_activity,
                    &iur.notes,
                    &shop_id,
                ],
            )
            .await?;
    }
    transaction.commit().await?;
    Ok(true)
}
