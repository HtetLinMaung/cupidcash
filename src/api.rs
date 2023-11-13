mod auth;
mod category;
mod image;
mod item;
mod order;
mod table;

use actix_web::web;

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(auth::login);
    cfg.service(auth::hash_password);
    cfg.service(category::get_categories);
    cfg.service(item::get_items);
    cfg.service(table::get_tables);
    cfg.service(order::create_order);
    cfg.service(order::get_orders);
    cfg.service(order::get_order_detail);
    cfg.service(auth::verify_token);
    cfg.service(order::get_order_by_id);
    cfg.service(order::update_order);
    cfg.service(item::add_item);
    cfg.service(item::get_item_by_id);
    cfg.service(item::update_item);
    cfg.service(item::delete_item);
}
