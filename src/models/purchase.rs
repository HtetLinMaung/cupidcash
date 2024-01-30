use chrono::{NaiveDate, NaiveDateTime};
use serde::{Deserialize, Serialize};
use tokio_postgres::{types::ToSql, Client, Error};

use crate::utils::{
    common_struct::PaginationResult,
    sql::{generate_pagination_query, PaginationOptions},
};

#[derive(Debug, Serialize, Deserialize)] // Add Debug derive
pub struct PurchaseDetail {
    pub purchase_detail_id: i32,
    pub ingredient_id: i32,
    pub ingredient_name: String,
    pub quantity_purchased: f32,
    pub unit: String,
    pub buying_price_per_unit: f32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Purchase {
    pub purchase_id: i32,
    pub total_cost: f64,
    pub purchase_date: NaiveDateTime,
    pub shop_id: i32,
    pub shop_name: String,
    pub created_at: NaiveDateTime,
    pub purchase_details: Vec<PurchaseDetail>,

}

pub async fn get_purchases(
    search: &Option<String>,
    page: Option<usize>,
    per_page: Option<usize>,
    client: &Client,
) -> Result<PaginationResult<Purchase>, Error> {
    let base_query =
        "from purchases p  join shops s on s.id = p.shop_id  where p.deleted_at is null".to_string();
    let params: Vec<Box<dyn ToSql + Sync>> = vec![];

    let order_options = "created_at desc";

    let result = generate_pagination_query(PaginationOptions {
        select_columns: "p.purchase_id,p.total_cost::text as total_cost, p.shop_id,s.name shop_name, p.purchase_date, p.created_at",
        base_query: &base_query,
        search_columns: vec!["p.purchase_id::varchar", "s.name"],
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
    let rows = client
        .query(&result.query, &params_slice)
        .await?;
        let mut purchases: Vec<Purchase> = vec![];
        for row in &rows {
            let purchase_id: i32 = row.get("purchase_id");
            let total_cost_str: &str = row.get("total_cost");
            let purchase_detail_rows =  client
            .query(
                "select pd.purchase_detail_id,pd.purchase_id,i.name as ingredient_name, pd.ingredient_id,pd.quantity_purchased::text as quantity_purchased,pd.unit,pd.buying_price_per_unit::text buying_price_per_unit
                from purchase_details pd 
                join purchases p on p.purchase_id = pd.purchase_id
                join ingredients i on i.ingredient_id = pd.ingredient_id
                where pd.purchase_id = $1 and p.deleted_at is null and i.deleted_at is null",
                &[&purchase_id],
            )
            .await?;
            purchases.push(
                Purchase {
                    purchase_id: purchase_id,
                    total_cost: total_cost_str.parse().unwrap(),
                    purchase_date: row.get("purchase_date"),
                    shop_id: row.get("shop_id"),
                    shop_name: row.get("shop_name"),
                    created_at: row.get("created_at"),
                    purchase_details: purchase_detail_rows
                        .iter()
                        .map(|row| {
                            let quantity_purchased_str: &str = row.get("quantity_purchased");
                            let buying_price_per_unit_str: &str = row.get("buying_price_per_unit");
                            return PurchaseDetail{
                                purchase_detail_id: row.get("purchase_detail_id"),
                                ingredient_id: row.get("ingredient_id"),
                                ingredient_name: row.get("ingredient_name"),
                                quantity_purchased: quantity_purchased_str.parse().unwrap(),
                                unit: row.get("unit"),
                                buying_price_per_unit:  buying_price_per_unit_str.parse().unwrap(),
                            }
                        
                        })
                        .collect(),

                }
            );
        }

    Ok(PaginationResult {
        data: purchases,
        total,
        page: current_page,
        per_page: limit,
        page_counts,
    })
}

#[derive(Debug, Deserialize)]
pub struct AddPurchaseRequest {
    pub total_cost: f64,
    pub purchase_date: NaiveDate,
    pub shop_id: i32,
    pub purchase_details: Vec<AddPurchaseDetailRequest>,
}

#[derive(Debug, Deserialize)]
pub struct AddPurchaseDetailRequest {
    pub ingredient_id: i32,
    pub quantity_purchased: f32,
    pub unit: String,
    pub buying_price_per_unit: f32,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePurchaseRequest {
    pub total_cost: f64,
    pub purchase_date: NaiveDate,
    pub shop_id: i32,
    pub purchase_details: Vec<UpdatePurchaseDetailRequest>,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePurchaseDetailRequest {
    pub purchase_detail_id: i32,
    pub ingredient_id: i32,
    pub quantity_purchased: f32,
    pub buying_price_per_unit: f32,
}

pub async fn add_purchase(
    data: &AddPurchaseRequest,
    client:  &mut Client,
) -> Result<(), Box<dyn std::error::Error>> {
    let transaction = client.transaction().await?;

   let purchase_insert_query = format!("insert into purchases (total_cost,purchase_date,shop_id) values ({},'{}',$1)  RETURNING purchase_id",data.total_cost,data.purchase_date.to_string());
   let purchase_id: i32 = transaction
   .query_one(
    &purchase_insert_query,
       &[&data.shop_id ],
   )
   .await?
   .get("purchase_id"); 
    for data in &data.purchase_details {
        let purchase_details_insert_query = format!("insert into purchase_details (purchase_id, ingredient_id, quantity_purchased, unit, buying_price_per_unit) values ($1,$2,{}, $3, {})",data.quantity_purchased,data.buying_price_per_unit);
        transaction.execute(&purchase_details_insert_query, &[&purchase_id,&data.ingredient_id,&data.unit]).await?;
        let ingredients_update_query = format!("update ingredients SET stock_quantity = stock_quantity + {} WHERE ingredient_id = $1 AND deleted_at IS NULL",data.quantity_purchased);
        transaction.execute(&ingredients_update_query,
            &[&data.ingredient_id],
        )
        .await?;
    }
    transaction.commit().await?;

    Ok(())
}

pub async fn get_purchase_by_id(purchase_id: i32, client: &Client) -> Option<Purchase> {
    let result = client.query_one("select p.purchase_id,p.total_cost::text as total_cost, p.purchase_date, p.shop_id, p.created_at,s.name as shop_name from purchases p join shops s on s.id = p.shop_id  where p.deleted_at is null  and purchase_id = $1 and s.deleted_at is null", &[&purchase_id]).await;
    let purchase_details_rows = match client
        .query(
            "select pd.purchase_detail_id,pd.purchase_id,i.name as ingredient_name, pd.ingredient_id,pd.quantity_purchased::text as quantity_purchased,pd.unit,pd.buying_price_per_unit::text as buying_price_per_unit
            from purchase_details pd 
            join purchases p on p.purchase_id = pd.purchase_id
			join ingredients i on i.ingredient_id = pd.ingredient_id
            where pd.purchase_id = $1 and p.deleted_at is null and i.deleted_at is null",
            &[&purchase_id],
        )
        .await
    {
        Ok(rows) => rows,
        Err(err) => {
            println!("{:?}", err);
            vec![]
        }
    };
    match result {
        Ok(row) => {
            let total_cost_str: &str = row.get("total_cost");

            Some(Purchase {
                purchase_id: row.get("purchase_id"),
                total_cost: total_cost_str.parse().unwrap(),
                purchase_date: row.get("purchase_date"),
                shop_id: row.get("shop_id"),
                shop_name: row.get("shop_name"),
                created_at: row.get("created_at"),
                purchase_details: purchase_details_rows
                        .iter()
                        .map(|row| {
                            let quantity_purchased_str: &str = row.get("quantity_purchased");
                            let buying_price_per_unit_str: &str = row.get("buying_price_per_unit");
                            return PurchaseDetail{
                                purchase_detail_id: row.get("purchase_detail_id"),
                                ingredient_id: row.get("ingredient_id"),
                                ingredient_name: row.get("ingredient_name"),
                                quantity_purchased: quantity_purchased_str.parse().unwrap(),
                                unit: row.get("unit"),
                                buying_price_per_unit:  buying_price_per_unit_str.parse().unwrap(),
                            }
                           
                        })
                        .collect(),
            })
        },
        Err(err) => {
            println!("{:?}", err);
            None
        }
        
    }
}

pub async fn update_purchase(
    data: &UpdatePurchaseRequest,
    purchase_id: i32,
    client: &mut Client,
) -> Result<(), Box<dyn std::error::Error>> {
    let transaction = client.transaction().await?;
    let query = format!("update purchases set total_cost = {}, purchase_date = '{}', shop_id = $1 where purchase_id = $2 and deleted_at is null",data.total_cost,data.purchase_date);
    transaction.execute(
            &query,
            &[
                &data.shop_id,
                &purchase_id,
            ],
        )
        .await?;
    // client.execute("delete from purchase_details where purchase_id = $1",&[&purchase_id],).await?;

    for purchase_detail  in &data.purchase_details {
        let purchase_details_update_query = format!("update purchase_details set ingredient_id = $1, quantity_purchased= {}, buying_price_per_unit = {} where purchase_detail_id = $2  and deleted_at is null",purchase_detail.quantity_purchased, purchase_detail.buying_price_per_unit);
        transaction.execute(&purchase_details_update_query, &[&purchase_detail.ingredient_id, &purchase_detail.purchase_detail_id])
            .await?;
        let ingredients_update_query = format!("update ingredients set stock_quantity = stock_quantity + 
        ( {} - (
        select quantity_purchased 
        from purchase_details 
        where purchase_detail_id = $1) ) where ingredient_id = $2 and deleted_at is null",purchase_detail.quantity_purchased);
        transaction.execute(&ingredients_update_query,
        &[&purchase_detail.purchase_detail_id,&purchase_detail.ingredient_id],
    )
    .await?;
    }
    transaction.commit().await?;
    Ok(())
}

pub async fn delete_purchase(
    purchase_id: i32,
    client: &Client,
) -> Result<(), Box<dyn std::error::Error>> {
    client
        .execute(
            "update purchases set deleted_at = CURRENT_TIMESTAMP where purchase_id = $1",
            &[&purchase_id],
        )
        .await?;
    client
    .execute(
        "update purchase_details set deleted_at = CURRENT_TIMESTAMP where purchase_id = $1",
        &[&purchase_id],
    )
    .await?;
    Ok(())
}



// pub async fn get_purchasedetails_by_id(purchase_id: i32,client: &mut Client,) -> Option<Purchase> {

//     let query = "select p.purchase_id,,pd.purchase_detail_id,pd.ingredient_id,pd.quantity_purchased
//     from purchases p join purchase_details pd on p.purchase_id = pd.purchase_id
//     where pd.purchase_id = $1 and p.deleted_at is null and pd.deleted_at is null";

//     let rows = client.query(query, &[&purchase_id]).await;

//     match rows {
//         Ok(rows) => {
//             for row in rows {
//                 // Access the columns of each row as needed
//                 let column1: i32 = row.get("column1");
//                 let column2: String = row.get("column2");

//                 // Do something with the data
//                 println!("Column1: {}, Column2: {}", column1, column2);
//             }
//         }
//         Err(err) => {
//             eprintln!("Error executing query: {}", err);
//         }
//     }
// }
