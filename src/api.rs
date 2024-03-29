mod auth;
mod category;
mod discount_type;
mod image;
mod ingredient_usage;
mod item;
mod order;
mod role;
mod shop;
mod table;
mod user;
mod ingredient;
mod purchashe;


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
    cfg.service(discount_type::get_discount_types);
    cfg.service(discount_type::add_discount_type);
    cfg.service(discount_type::get_discount_type_by_id);
    cfg.service(discount_type::update_discount_type);
    cfg.service(discount_type::delete_discount_type);
    cfg.service(order::get_daily_sale_report);
    // cfg.service(order::download_daily_sale_report);
    cfg.service(ingredient::get_ingredients);
    cfg.service(ingredient::add_ingredient);
    cfg.service(ingredient::get_ingredient_by_id);
    cfg.service(ingredient::update_ingredient);
    cfg.service(ingredient::delete_ingredient);
    cfg.service(purchashe::get_purchases);
    cfg.service(purchashe::add_purchase);
    cfg.service(purchashe::get_purchase_by_id);
    cfg.service(purchashe::update_purchase);
    cfg.service(purchashe::delete_purchase);
    cfg.service(ingredient_usage::add_ingredient_usages);
    cfg.service(order::daily_sale_report_pdf);
    cfg.service(order::daily_sale_report_excel);
    cfg.service(ingredient_usage::get_ingredient_usages);
    cfg.service(ingredient_usage::get_ingredient_usage_by_id);
    cfg.service(ingredient_usage::update_ingredient_usage);
    cfg.service(ingredient_usage::delete_ingredient_usage);
}
