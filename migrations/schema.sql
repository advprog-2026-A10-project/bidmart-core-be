-- Create enum types
DO $$ BEGIN
    CREATE TYPE listing_status AS ENUM ('DRAFT', 'ACTIVE', 'EXTENDED', 'CLOSED', 'WON', 'UNSOLD');
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;

DO $$ BEGIN
    CREATE TYPE transaction_type AS ENUM ('TOPUP', 'WITHDRAW', 'HOLD', 'RELEASE', 'PAYMENT');
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;

DO $$ BEGIN
    CREATE TYPE shipping_status AS ENUM ('PACKED', 'SHIPPED', 'DELIVERED');
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;

DO $$ BEGIN
    CREATE TYPE notification_type AS ENUM ('BID_OUTBID', 'AUCTION_WON', 'AUCTION_CLOSED', 'ORDER_UPDATE', 'PAYMENT_SUCCESS', 'SYSTEM_ALERT');
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;

-- Create Categories table
CREATE TABLE IF NOT EXISTS categories (
    id SERIAL PRIMARY KEY,
    parent_id INTEGER REFERENCES categories(id),
    name VARCHAR(255) NOT NULL,
    slug VARCHAR(255) NOT NULL UNIQUE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- Create Listings table
CREATE TABLE IF NOT EXISTS listings (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    seller_id UUID NOT NULL,
    category_id INTEGER REFERENCES categories(id),
    title VARCHAR(255) NOT NULL,
    description TEXT,
    start_price DECIMAL(12, 2) NOT NULL,
    reserve_price DECIMAL(12, 2),
    current_price DECIMAL(12, 2) NOT NULL,
    min_increment DECIMAL(12, 2) NOT NULL DEFAULT 1.00,
    DEFAULT 'DRAFT status LISTING_STATUS',
    ends_at TIMESTAMP WITH TIME ZONE NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- Create bids table
CREATE TABLE IF NOT EXISTS bids (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    listing_id UUID NOT NULL REFERENCES listings(id),
    buyer_id UUID NOT NULL,
    amount DECIMAL(12, 2) NOT NULL,
    is_proxy_bid BOOLEAN DEFAULT false,
    max_proxy_amount DECIMAL(12, 2),
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- Create wallets table
CREATE TABLE IF NOT EXISTS wallets (
    user_id UUID PRIMARY KEY,
    balance DECIMAL(12, 2) DEFAULT 0,
    held_balance DECIMAL(12, 2) DEFAULT 0,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- Create wallet transactions table
CREATE TABLE IF NOT EXISTS wallet_transactions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    wallet_id UUID NOT NULL REFERENCES wallets(user_id),
    type TRANSACTION_TYPE NOT NULL,
    amount DECIMAL(12, 2) NOT NULL,
    reference_id UUID,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- Create orders table
CREATE TABLE IF NOT EXISTS orders (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    listing_id UUID NOT NULL REFERENCES listings(id),
    buyer_id UUID NOT NULL,
    seller_id UUID NOT NULL,
    final_price DECIMAL(12, 2) NOT NULL,
    shipping_status SHIPPING_STATUS,
    tracking_number VARCHAR(100),
    is_disputed BOOLEAN DEFAULT false,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- Create notifications table
CREATE TABLE IF NOT EXISTS notifications (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL,
    type NOTIFICATION_TYPE NOT NULL,
    title VARCHAR(255) NOT NULL,
    message TEXT NOT NULL,
    is_read BOOLEAN DEFAULT false,
    reference_id UUID,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- Create indexes
CREATE INDEX IF NOT EXISTS idx_categories_parent_id ON categories(parent_id);
CREATE INDEX IF NOT EXISTS idx_categories_slug ON categories(slug);

CREATE INDEX IF NOT EXISTS idx_listings_seller_id ON listings(seller_id);
CREATE INDEX IF NOT EXISTS idx_listings_category_id ON listings(category_id);
CREATE INDEX IF NOT EXISTS idx_listings_status ON listings(status);
CREATE INDEX IF NOT EXISTS idx_listings_ends_at ON listings(ends_at);

CREATE INDEX IF NOT EXISTS idx_bids_listing_id ON bids(listing_id);
CREATE INDEX IF NOT EXISTS idx_bids_buyer_id ON bids(buyer_id);
CREATE INDEX IF NOT EXISTS idx_bids_amount ON bids(amount);
CREATE INDEX IF NOT EXISTS idx_bids_created_at ON bids(created_at);

CREATE INDEX IF NOT EXISTS idx_wallet_transactions_wallet_id ON wallet_transactions(wallet_id);
CREATE INDEX IF NOT EXISTS idx_wallet_transactions_type ON wallet_transactions(type);
CREATE INDEX IF NOT EXISTS idx_wallet_transactions_created_at ON wallet_transactions(created_at);

CREATE INDEX IF NOT EXISTS idx_orders_listing_id ON orders(listing_id);
CREATE INDEX IF NOT EXISTS idx_orders_buyer_id ON orders(buyer_id);
CREATE INDEX IF NOT EXISTS idx_orders_seller_id ON orders(seller_id);
CREATE INDEX IF NOT EXISTS idx_orders_shipping_status ON orders(shipping_status);

CREATE INDEX IF NOT EXISTS idx_notifications_user_id ON notifications(user_id);
CREATE INDEX IF NOT EXISTS idx_notifications_user_read ON notifications(user_id, is_read);
CREATE INDEX IF NOT EXISTS idx_notifications_created_at ON notifications(created_at);
