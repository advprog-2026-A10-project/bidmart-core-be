-- Add migration script here
CREATE EXTENSION IF NOT EXISTS "pgcrypto";

DO $$ BEGIN
  CREATE TYPE listing_status AS ENUM ('DRAFT','ACTIVE','EXTENDED','CLOSED','WON','UNSOLD');
EXCEPTION WHEN duplicate_object THEN NULL; END $$;

DO $$ BEGIN
  CREATE TYPE transaction_type AS ENUM ('TOPUP','WITHDRAW','HOLD','RELEASE','PAYMENT');
EXCEPTION WHEN duplicate_object THEN NULL; END $$;

DO $$ BEGIN
  CREATE TYPE shipping_status AS ENUM ('PACKED','SHIPPED','DELIVERED');
EXCEPTION WHEN duplicate_object THEN NULL; END $$;

CREATE TABLE IF NOT EXISTS categories (
  id integer GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  parent_id integer REFERENCES categories(id) ON DELETE SET NULL,
  name varchar NOT NULL,
  slug varchar UNIQUE NOT NULL
);

CREATE TABLE IF NOT EXISTS listings (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  seller_id uuid NOT NULL,
  category_id integer REFERENCES categories(id),
  title varchar NOT NULL,
  description text,
  start_price numeric(18,2) NOT NULL CHECK (start_price >= 0),
  reserve_price numeric(18,2) CHECK (reserve_price IS NULL OR reserve_price >= 0),
  current_price numeric(18,2) NOT NULL DEFAULT 0 CHECK (current_price >= 0),
  min_increment numeric(18,2) NOT NULL DEFAULT 0 CHECK (min_increment >= 0),
  status listing_status NOT NULL DEFAULT 'DRAFT',
  ends_at timestamptz,
  created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS bids (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  listing_id uuid NOT NULL REFERENCES listings(id) ON DELETE CASCADE,
  buyer_id uuid NOT NULL,
  amount numeric(18,2) NOT NULL CHECK (amount > 0),
  is_proxy_bid boolean NOT NULL DEFAULT false,
  max_proxy_amount numeric(18,2),
  created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS wallets (
  user_id uuid PRIMARY KEY,
  balance numeric(18,2) NOT NULL DEFAULT 0 CHECK (balance >= 0),
  held_balance numeric(18,2) NOT NULL DEFAULT 0 CHECK (held_balance >= 0),
  updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS wallet_transactions (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  wallet_id uuid NOT NULL REFERENCES wallets(user_id) ON DELETE CASCADE,
  type transaction_type NOT NULL,
  amount numeric(18,2) NOT NULL CHECK (amount >= 0),
  reference_id uuid,
  created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS orders (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  listing_id uuid NOT NULL REFERENCES listings(id),
  buyer_id uuid NOT NULL,
  seller_id uuid NOT NULL,
  final_price numeric(18,2) NOT NULL CHECK (final_price >= 0),
  shipping_status shipping_status,
  tracking_number varchar,
  is_disputed boolean NOT NULL DEFAULT false,
  created_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_bids_listing_id ON bids(listing_id);
CREATE INDEX IF NOT EXISTS idx_wallet_tx_wallet_id ON wallet_transactions(wallet_id);
CREATE INDEX IF NOT EXISTS idx_orders_listing_id ON orders(listing_id);