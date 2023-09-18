-- Shops Table
CREATE TABLE shops (
    id SERIAL PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    address TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Roles Table
CREATE TABLE roles (
    id SERIAL PRIMARY KEY,
    role_name VARCHAR(50) NOT NULL UNIQUE
);

-- Insert roles into the Roles Table
INSERT INTO roles (role_name) VALUES ('Admin'), ('Manager'), ('Waiter');

-- Users Table
CREATE TABLE users (
    id SERIAL PRIMARY KEY,
    username VARCHAR(100) UNIQUE NOT NULL,
    password TEXT NOT NULL,  -- ensure it's hashed in application logic
    role_id INTEGER REFERENCES roles(id),
    shop_id INTEGER REFERENCES shops(id),
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Categories Table
CREATE TABLE categories (
    id SERIAL PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    shop_id INTEGER REFERENCES shops(id),
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Items (Menu Items) Table with category reference and image
CREATE TABLE items (
    id SERIAL PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    price DECIMAL(10, 2) NOT NULL,
    category_id INTEGER REFERENCES categories(id),
    image_url TEXT,  -- This can store the URL of the image if hosted externally
    shop_id INTEGER REFERENCES shops(id),
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);


-- Tables (Restaurant Tables) Table
CREATE TABLE tables (
    id SERIAL PRIMARY KEY,
    table_number INTEGER NOT NULL,
    qr_code TEXT,  -- if storing QR code data
    shop_id INTEGER REFERENCES shops(id),
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Orders Table
CREATE TABLE orders (
    id SERIAL PRIMARY KEY,
    waiter_id INTEGER REFERENCES users(id),
    table_id INTEGER REFERENCES tables(id),
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Order Items (link between orders and items) Table
CREATE TABLE order_items (
    order_id INTEGER REFERENCES orders(id),
    item_id INTEGER REFERENCES items(id),
    quantity INTEGER NOT NULL,
    special_instructions TEXT,
    PRIMARY KEY(order_id, item_id)
);

-- Transaction Reports (for simplicity, assuming aggregate reports) Table
CREATE TABLE transaction_reports (
    id SERIAL PRIMARY KEY,
    shop_id INTEGER REFERENCES shops(id),
    date DATE NOT NULL,
    total_revenue DECIMAL(10, 2),
    num_orders INTEGER,
    popular_item_id INTEGER REFERENCES items(id),
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Future tables (like feedback, loyalty programs) can be added based on requirements

