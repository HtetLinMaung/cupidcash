mod auth;
mod category;
mod image;
mod item;
mod order;
mod role;
mod shop;
mod table;
mod user;

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
    cfg.service(user::add_user);
    cfg.service(user::get_users);
    cfg.service(user::get_user_by_id);
    cfg.service(user::update_user);
    cfg.service(user::delete_user);
    cfg.service(role::get_roles);
    cfg.service(category::add_category);
    cfg.service(category::get_category_by_id);
    cfg.service(category::update_category);
    cfg.service(category::delete_category);
    cfg.service(shop::add_shop);
    cfg.service(shop::get_shops);
    cfg.service(shop::get_shop_by_id);
    cfg.service(shop::update_shop);
    cfg.service(shop::delete_shop);
    cfg.service(table::add_table);
    cfg.service(table::get_table_by_id);
    cfg.service(table::update_table);
    cfg.service(table::delete_table);
    cfg.service(image::upload);
}
