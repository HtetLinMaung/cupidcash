-- Shops Table
CREATE TABLE shops
(
    id SERIAL PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    address TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    deleted_at TIMESTAMP DEFAULT null
);

INSERT INTO shops
    (name, address, created_at)
VALUES
    ('Café Central', '123 Main St, CityCenter, CountryXYZ', '2023-09-20 10:00:00'),
    ('Bistro Corner', '456 Oak Road, Suburbia, CountryXYZ', '2023-09-15 11:30:00'),
    ('Green Delight Restaurant', '789 Pine Avenue, GreenValley, CountryXYZ', '2023-09-01 09:00:00'),
    ('Harbor Cafe', '101 Seaside Blvd, Beachtown, CountryXYZ', '2023-08-25 14:00:00'),
    ('Mountain Brews', '202 Hilltop Drive, Highland, CountryXYZ', '2023-09-10 08:00:00');


-- Roles Table
CREATE TABLE roles
(
    id SERIAL PRIMARY KEY,
    role_name VARCHAR(50) NOT NULL UNIQUE,
    deleted_at TIMESTAMP DEFAULT null
);

-- Insert roles into the Roles Table
INSERT INTO roles
    (role_name)
VALUES
    ('Admin'),
    ('Manager'),
    ('Waiter');

-- Users Table
CREATE TABLE users
(
    id SERIAL PRIMARY KEY,
    name varchar(255) not null,
    username VARCHAR(100) UNIQUE NOT NULL,
    password TEXT NOT NULL,
    role_id INTEGER REFERENCES roles(id),
    shop_id INTEGER REFERENCES shops(id),
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    deleted_at TIMESTAMP DEFAULT null
);
insert into users
    (username, password, name, role_id, shop_id, created_at)
values
    ('waiter001', '$2b$12$ZQ9xFoR3ve/kjfoMeGuoLO93USM5q.z08or2Z2HMuH0BlS7bTfyTm', 'Waiter 001', 3, 2, now());
insert into users
    (username, password, name, role_id, created_at)
values
    ('admin', '$2b$12$mPXBoB9P8Csv9MWc89YAnOkVzi.g5YiiLgPizF0vnIV3.Ckyr5SfG', 'Admin', 1, now());

-- Categories Table
CREATE TABLE categories
(
    id SERIAL PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    shop_id INTEGER REFERENCES shops(id),
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    deleted_at TIMESTAMP DEFAULT null
);

INSERT INTO categories
    (name, description, shop_id, created_at)
VALUES
    ('Appetizers', 'Start your meal off right with our selection of appetizers.', 2, '2023-09-15 12:00:00'),
    ('Main Courses', 'From steaks to pasta, explore our variety of hearty dishes.', 2, '2023-09-15 12:10:00'),
    ('Desserts', 'Satisfy your sweet tooth with our delicious desserts.', 2, '2023-09-15 12:20:00'),
    ('Beverages', 'Quench your thirst with our range of drinks.', 2, '2023-09-15 12:30:00'),
    ('Vegan Options', 'A selection of dishes curated for our vegan patrons.', 2, '2023-09-15 12:40:00');


-- Items (Menu Items) Table with category reference and image
CREATE TABLE items
(
    id SERIAL PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    price DECIMAL(10, 2) NOT NULL,
    image_url TEXT,
    shop_id INTEGER REFERENCES shops(id),
    discount_percent DECIMAL(10, 2) DEFAULT 0,
    discount_expiration TIMESTAMP DEFAULT null,
    discount_reason TEXT DEFAULT '',
    discounted_price DECIMAL(18, 2) DEFAULT 0.0,
    discount_type VARCHAR(255) DEFAULT 'No Discount',
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    deleted_at TIMESTAMP DEFAULT null
);

-- Sample data for items table
INSERT INTO items
    (name, description, price, image_url, shop_id)
VALUES
    ('Espresso', 'Strong coffee without milk', 2.50, '/images/espresso.jpeg', 2),
    ('Cappuccino', 'Coffee with frothy milk on top', 3.00, '/images/espresso.jpeg', 2),
    ('Green Salad', 'Mixed greens with vinaigrette', 5.50, '/images/espresso.jpeg', 2),
    ('Cheese Burger', 'Burger with cheese and lettuce', 7.00, '/images/espresso.jpeg', 2),
    ('Spaghetti Carbonara', 'Creamy pasta with bacon bits', 8.50, '/images/espresso.jpeg', 2),
    ('Lemon Tart', 'Tangy lemon dessert', 4.50, '/images/espresso.jpeg', 2),
    ('Mineral Water', 'Sparkling water in a bottle', 1.50, '/images/espresso.jpeg', 2);

CREATE TABLE item_categories
(
    id SERIAL PRIMARY KEY,
    item_id INTEGER REFERENCES items(id),
    category_id INTEGER REFERENCES categories(id),
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

insert into item_categories
    (item_id, category_id)
values
    (1, 4),
    (2, 4),
    (3, 4),
    (4, 4),
    (5, 4),
    (6, 4),
    (7, 4);

-- Tables (Restaurant Tables) Table
CREATE TABLE tables
(
    id SERIAL PRIMARY KEY,
    table_number VARCHAR(255) NOT NULL,
    qr_code TEXT,
    shop_id INTEGER REFERENCES shops(id),
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    deleted_at TIMESTAMP DEFAULT null
);

INSERT INTO tables
    (table_number, qr_code, shop_id, created_at)
VALUES
    ('A1', '', 2, '2023-09-19 10:00:00'),
    ('A2', '', 2, '2023-09-19 10:05:00'),
    ('A3', '', 2, '2023-09-19 10:10:00'),
    ('A4', '', 2, '2023-09-19 10:15:00'),
    ('A5', '', 2, '2023-09-19 10:20:00'),
    ('A6', '', 2, '2023-09-19 10:25:00'),
    ('A7', '', 2, '2023-09-19 10:30:00'),
    ('A8', '', 2, '2023-09-19 10:35:00'),
    ('A9', '', 2, '2023-09-19 10:40:00'),
    ('A10', '', 2, '2023-09-19 10:45:00');

-- Orders Table
CREATE TABLE orders
(
    id SERIAL PRIMARY KEY,
    waiter_id INTEGER REFERENCES users(id),
    table_id INTEGER REFERENCES tables(id),
    status VARCHAR(50) DEFAULT 'Pending',
    discount DECIMAL(10, 2) DEFAULT 0.0,
    tax DECIMAL(10, 2) DEFAULT 0.0,
    total DECIMAL(10, 2) DEFAULT 0.0,
    payment_type VARCHAR(10) DEFAULT 'CASH',
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    deleted_at TIMESTAMP DEFAULT null
);

-- Order Items (link between orders and items) Table
CREATE TABLE order_items
(
    order_id INTEGER REFERENCES orders(id),
    item_id INTEGER REFERENCES items(id),
    price DECIMAL(10, 2) DEFAULT 0.0,
    quantity INTEGER NOT NULL,
    special_instructions TEXT,
    original_price DECIMAL(10, 2) DEFAULT 0.0,
    PRIMARY KEY(order_id, item_id)
);

-- Transaction Reports (for simplicity, assuming aggregate reports) Table
CREATE TABLE transaction_reports
(
    id SERIAL PRIMARY KEY,
    shop_id INTEGER REFERENCES shops(id),
    date DATE NOT NULL,
    total_revenue DECIMAL(10, 2),
    num_orders INTEGER,
    popular_item_id INTEGER REFERENCES items(id),
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    deleted_at TIMESTAMP DEFAULT null
);

CREATE TABLE discount_types
(
    discount_type_id SERIAL PRIMARY KEY,
    description VARCHAR(255) NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    deleted_at TIMESTAMP DEFAULT NULL
);

INSERT INTO discount_types (description) 
VALUES ('No Discount'),('Discount by Specific Percentage'),
('Discount by Specific Amount');
-- Future tables (like feedback, loyalty programs) can be added based on requirements

