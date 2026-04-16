-- ============================================================
-- ENUM TYPES
-- ============================================================
DO $$ BEGIN
    CREATE TYPE listing_status AS ENUM (
        'DRAFT',
        'ACTIVE',
        'SOLD',
        'CANCELLED',
        'EXPIRED'
    );
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;
DO $$ BEGIN
    CREATE TYPE auction_status AS ENUM (
        'DRAFT',
        'SCHEDULED',
        'ACTIVE',
        'EXTENDED',
        'CLOSED',
        'WON',
        'UNSOLD'
    );
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;
DO $$ BEGIN
    CREATE TYPE order_status AS ENUM (
        'PENDING_PAYMENT',
        'PAID',
        'SHIPPED',
        'DELIVERED',
        'CONFIRMED',
        'DISPUTED',
        'REFUNDED',
        'CANCELLED'
    );
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;
DO $$ BEGIN
    CREATE TYPE shipping_status AS ENUM (
        'PACKED',
        'SHIPPED',
        'DELIVERED'
    );
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;
DO $$ BEGIN
    CREATE TYPE transaction_type AS ENUM (
        'TOPUP',
        'WITHDRAW',
        'BID_HOLD',
        'BID_RELEASE',
        'BID_CONVERT',
        'PAYMENT_RECEIVED',
        'REFUND'
    );
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;
DO $$ BEGIN
    CREATE TYPE transaction_status AS ENUM (
        'PENDING',
        'COMPLETED',
        'FAILED',
        'CANCELLED'
    );
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;
DO $$ BEGIN
    CREATE TYPE notification_type AS ENUM (
        'BID_OUTBID',
        'AUCTION_WON',
        'AUCTION_LOST',
        'ORDER_SHIPPED',
        'ORDER_DELIVERED',
        'PAYMENT_RECEIVED',
        'DISPUTE_OPENED',
        'DISPUTE_RESOLVED',
        'AUCTION_EXTENDED'
    );
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;
DO $$ BEGIN
    CREATE TYPE dispute_status AS ENUM (
        'OPEN',
        'UNDER_REVIEW',
        'RESOLVED_BUYER',
        'RESOLVED_SELLER',
        'CLOSED'
    );
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;
DO $$ BEGIN
    CREATE TYPE dispute_reason AS ENUM (
        'ITEM_NOT_AS_DESCRIBED',
        'ITEM_NOT_RECEIVED',
        'OTHER'
    );
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;
DO $$ BEGIN
    CREATE TYPE reference_type AS ENUM (
        'auction',
        'order',
        'dispute',
        'topup',
        'withdraw'
    );
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;

-- ============================================================
-- CATEGORIES
-- ============================================================
CREATE TABLE IF NOT EXISTS categories (
    id          SERIAL PRIMARY KEY,
    parent_id   INTEGER REFERENCES categories(id) ON DELETE SET NULL,
    name        VARCHAR(255) NOT NULL,
    slug        VARCHAR(255) NOT NULL UNIQUE,
    image_url   TEXT,
    child_count INTEGER NOT NULL DEFAULT 0,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);
-- ============================================================
-- LISTINGS
-- ============================================================
CREATE TABLE IF NOT EXISTS listings (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    seller_id           UUID NOT NULL,
    seller_name         VARCHAR(255) NOT NULL,
    category_id         INTEGER REFERENCES categories(id) ON DELETE SET NULL,
    category_name       VARCHAR(255) NOT NULL,
    title               VARCHAR(255) NOT NULL,
    description         TEXT NOT NULL DEFAULT '',
    start_price         BIGINT NOT NULL,
    reserve_price       BIGINT,
    current_price       BIGINT NOT NULL,
    min_increment       BIGINT NOT NULL DEFAULT 100,
    bid_count           INTEGER NOT NULL DEFAULT 0,
    status              listing_status NOT NULL DEFAULT 'DRAFT',
    auction_id          UUID,                      -- FK added after auctions table
    starts_at           TIMESTAMPTZ NOT NULL,
    ends_at             TIMESTAMPTZ NOT NULL,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);
-- ============================================================
-- LISTING IMAGES
-- ============================================================
CREATE TABLE IF NOT EXISTS listing_images (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    listing_id  UUID NOT NULL REFERENCES listings(id) ON DELETE CASCADE,
    url         TEXT NOT NULL,
    "order"     INTEGER NOT NULL DEFAULT 0,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);
-- ============================================================
-- AUCTIONS
-- ============================================================
CREATE TABLE IF NOT EXISTS auctions (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    listing_id          UUID NOT NULL REFERENCES listings(id) ON DELETE CASCADE,
    seller_id           UUID NOT NULL,
    seller_name         VARCHAR(255) NOT NULL,
    title               VARCHAR(255) NOT NULL,
    description         TEXT NOT NULL DEFAULT '',
    image_url           TEXT NOT NULL DEFAULT '',
    start_price         BIGINT NOT NULL,
    current_price       BIGINT NOT NULL,
    reserve_price       BIGINT,
    bid_increment       BIGINT NOT NULL DEFAULT 100,
    bid_count           INTEGER NOT NULL DEFAULT 0,
    status              auction_status NOT NULL DEFAULT 'DRAFT',
    winner_id           UUID,
    winner_name         VARCHAR(255),
    starts_at           TIMESTAMPTZ NOT NULL,
    ends_at             TIMESTAMPTZ NOT NULL,
    original_ends_at    TIMESTAMPTZ NOT NULL,
    extension_count     INTEGER NOT NULL DEFAULT 0,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT chk_winner CHECK (
        (winner_id IS NULL AND winner_name IS NULL) OR
        (winner_id IS NOT NULL AND winner_name IS NOT NULL)
    )
);
-- Back-fill FK from listings -> auctions
ALTER TABLE listings
    ADD CONSTRAINT fk_listings_auction
    FOREIGN KEY (auction_id) REFERENCES auctions(id) ON DELETE SET NULL;
-- ============================================================
-- BIDS
-- ============================================================
CREATE TABLE IF NOT EXISTS bids (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    auction_id      UUID NOT NULL REFERENCES auctions(id) ON DELETE CASCADE,
    bidder_id       UUID NOT NULL,
    bidder_name     VARCHAR(255) NOT NULL,
    amount          BIGINT NOT NULL,
    is_proxy        BOOLEAN NOT NULL DEFAULT false,
    is_winning      BOOLEAN NOT NULL DEFAULT false,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);
-- ============================================================
-- PROXY BIDS
-- ============================================================
CREATE TABLE IF NOT EXISTS proxy_bids (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    auction_id      UUID NOT NULL REFERENCES auctions(id) ON DELETE CASCADE,
    bidder_id       UUID NOT NULL,
    max_amount      BIGINT NOT NULL,
    is_active       BOOLEAN NOT NULL DEFAULT true,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT uq_proxy_bid_active UNIQUE (auction_id, bidder_id, is_active)
);
-- ============================================================
-- WALLETS
-- ============================================================
CREATE TABLE IF NOT EXISTS wallets (
    user_id         UUID PRIMARY KEY,
    balance         BIGINT NOT NULL DEFAULT 0,
    held_balance    BIGINT NOT NULL DEFAULT 0,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT chk_held_lte_balance CHECK (held_balance <= balance)
);
-- ============================================================
-- WALLET TRANSACTIONS
-- ============================================================
CREATE TABLE IF NOT EXISTS wallet_transactions (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    wallet_id       UUID NOT NULL REFERENCES wallets(user_id) ON DELETE CASCADE,
    type            transaction_type NOT NULL,
    status          transaction_status NOT NULL DEFAULT 'PENDING',
    amount          BIGINT NOT NULL,
    balance_after   BIGINT NOT NULL,
    reference_id    UUID,
    reference_type  reference_type,
    description     TEXT NOT NULL DEFAULT '',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    completed_at    TIMESTAMPTZ,
    CONSTRAINT chk_reference CHECK (
        (reference_id IS NULL AND reference_type IS NULL) OR
        (reference_id IS NOT NULL AND reference_type IS NOT NULL)
    )
);
-- ============================================================
-- ORDERS
-- ============================================================
CREATE TABLE IF NOT EXISTS orders (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    auction_id      UUID NOT NULL REFERENCES auctions(id),
    listing_id      UUID NOT NULL REFERENCES listings(id),
    buyer_id        UUID NOT NULL,
    buyer_name      VARCHAR(255) NOT NULL,
    seller_id       UUID NOT NULL,
    seller_name     VARCHAR(255) NOT NULL,
    title           VARCHAR(255) NOT NULL,
    image_url       TEXT NOT NULL DEFAULT '',
    final_price     BIGINT NOT NULL,
    status          order_status NOT NULL DEFAULT 'PENDING_PAYMENT',

    shipping_status shipping_status,
    carrier         VARCHAR(100),
    tracking_number VARCHAR(100),
    estimated_delivery_at TIMESTAMPTZ,

    paid_at         TIMESTAMPTZ,
    shipped_at      TIMESTAMPTZ,
    delivered_at    TIMESTAMPTZ,
    confirmed_at    TIMESTAMPTZ,
    is_disputed     BOOLEAN NOT NULL DEFAULT false,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);
-- ============================================================
-- DISPUTES
-- ============================================================
CREATE TABLE IF NOT EXISTS disputes (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    order_id    UUID NOT NULL REFERENCES orders(id) ON DELETE CASCADE,
    opened_by   UUID NOT NULL,
    reason      dispute_reason NOT NULL,
    description TEXT NOT NULL,
    status      dispute_status NOT NULL DEFAULT 'OPEN',
    resolution  TEXT,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    resolved_at TIMESTAMPTZ,
    CONSTRAINT uq_dispute_per_order UNIQUE (order_id)
);
-- ============================================================
-- NOTIFICATIONS
-- ============================================================
CREATE TABLE IF NOT EXISTS notifications (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID NOT NULL,
    type            notification_type NOT NULL,
    title           VARCHAR(255) NOT NULL,
    message         TEXT NOT NULL,
    is_read         BOOLEAN NOT NULL DEFAULT false,
    reference_id    UUID,
    reference_type  reference_type,
    read_at         TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);
-- ============================================================
-- INDEXES
-- ============================================================
-- categories
CREATE INDEX IF NOT EXISTS idx_categories_parent_id ON categories(parent_id);
CREATE INDEX IF NOT EXISTS idx_categories_slug ON categories(slug);
-- listings
CREATE INDEX IF NOT EXISTS idx_listings_seller_id ON listings(seller_id);
CREATE INDEX IF NOT EXISTS idx_listings_category_id ON listings(category_id);
CREATE INDEX IF NOT EXISTS idx_listings_status ON listings(status);
CREATE INDEX IF NOT EXISTS idx_listings_ends_at ON listings(ends_at);
CREATE INDEX IF NOT EXISTS idx_listings_auction_id ON listings(auction_id);
-- listing_images
CREATE INDEX IF NOT EXISTS idx_listing_images_listing_id ON listing_images(listing_id);
CREATE INDEX IF NOT EXISTS idx_listing_images_order ON listing_images(listing_id, "order");
-- auctions
CREATE INDEX IF NOT EXISTS idx_auctions_listing_id ON auctions(listing_id);
CREATE INDEX IF NOT EXISTS idx_auctions_seller_id ON auctions(seller_id);
CREATE INDEX IF NOT EXISTS idx_auctions_status ON auctions(status);
CREATE INDEX IF NOT EXISTS idx_auctions_ends_at ON auctions(ends_at);
CREATE INDEX IF NOT EXISTS idx_auctions_winner_id ON auctions(winner_id);
-- bids
CREATE INDEX IF NOT EXISTS idx_bids_auction_id ON bids(auction_id);
CREATE INDEX IF NOT EXISTS idx_bids_bidder_id ON bids(bidder_id);
CREATE INDEX IF NOT EXISTS idx_bids_amount ON bids(amount);
CREATE INDEX IF NOT EXISTS idx_bids_created_at ON bids(created_at);
CREATE INDEX IF NOT EXISTS idx_bids_winning ON bids(auction_id, is_winning);
-- proxy_bids
CREATE INDEX IF NOT EXISTS idx_proxy_bids_auction_id ON proxy_bids(auction_id);
CREATE INDEX IF NOT EXISTS idx_proxy_bids_bidder_id ON proxy_bids(bidder_id);
CREATE INDEX IF NOT EXISTS idx_proxy_bids_active ON proxy_bids(auction_id, is_active);
-- wallets
CREATE INDEX IF NOT EXISTS idx_wallets_user_id ON wallets(user_id);
-- wallet_transactions
CREATE INDEX IF NOT EXISTS idx_wallet_transactions_wallet_id ON wallet_transactions(wallet_id);
CREATE INDEX IF NOT EXISTS idx_wallet_transactions_type ON wallet_transactions(type);
CREATE INDEX IF NOT EXISTS idx_wallet_transactions_status ON wallet_transactions(status);
CREATE INDEX IF NOT EXISTS idx_wallet_transactions_reference ON wallet_transactions(reference_id, reference_type);
CREATE INDEX IF NOT EXISTS idx_wallet_transactions_created_at ON wallet_transactions(created_at);
-- orders
CREATE INDEX IF NOT EXISTS idx_orders_auction_id ON orders(auction_id);
CREATE INDEX IF NOT EXISTS idx_orders_listing_id ON orders(listing_id);
CREATE INDEX IF NOT EXISTS idx_orders_buyer_id ON orders(buyer_id);
CREATE INDEX IF NOT EXISTS idx_orders_seller_id ON orders(seller_id);
CREATE INDEX IF NOT EXISTS idx_orders_status ON orders(status);
CREATE INDEX IF NOT EXISTS idx_orders_shipping_status ON orders(shipping_status);
-- disputes
CREATE INDEX IF NOT EXISTS idx_disputes_order_id ON disputes(order_id);
CREATE INDEX IF NOT EXISTS idx_disputes_opened_by ON disputes(opened_by);
CREATE INDEX IF NOT EXISTS idx_disputes_status ON disputes(status);
-- notifications
CREATE INDEX IF NOT EXISTS idx_notifications_user_id ON notifications(user_id);
CREATE INDEX IF NOT EXISTS idx_notifications_user_read ON notifications(user_id, is_read);
CREATE INDEX IF NOT EXISTS idx_notifications_created_at ON notifications(created_at);
CREATE INDEX IF NOT EXISTS idx_notifications_reference ON notifications(reference_id, reference_type);