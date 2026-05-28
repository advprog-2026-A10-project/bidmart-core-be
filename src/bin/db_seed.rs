use anyhow::{Context, Result};
use bidmart_core_be::infrastructure::config::AppConfig;
use bidmart_core_be::infrastructure::database::create_pool;
use bidmart_core_be::infrastructure::database::migrations::run_pending_migrations;
use bidmart_core_be::infrastructure::logger::init_tracer;
use chrono::{Duration, Utc};
use sqlx::PgPool;
use std::collections::HashMap;
use uuid::Uuid;

struct CategorySeed {
    name: &'static str,
    slug: &'static str,
    parent_slug: Option<&'static str>,
    image_url: &'static str,
}

struct ListingSeed {
    id: &'static str,
    seller_id: &'static str,
    seller_name: &'static str,
    category_slug: &'static str,
    title: &'static str,
    description: &'static str,
    start_price: i64,
    reserve_price: Option<i64>,
    current_price: i64,
    min_increment: i64,
    bid_count: i32,
    starts_hours_ago: i64,
    ends_in_hours: i64,
    images: &'static [&'static str],
}

const CATEGORIES: &[CategorySeed] = &[
    CategorySeed {
        name: "Elektronik",
        slug: "elektronik",
        parent_slug: None,
        image_url: "https://placehold.co/640x360/png?text=Kategori+Elektronik",
    },
    CategorySeed {
        name: "Fashion",
        slug: "fashion",
        parent_slug: None,
        image_url: "https://placehold.co/640x360/png?text=Kategori+Fashion",
    },
    CategorySeed {
        name: "Rumah Tangga",
        slug: "rumah-tangga",
        parent_slug: None,
        image_url: "https://placehold.co/640x360/png?text=Kategori+Rumah+Tangga",
    },
    CategorySeed {
        name: "Hobi",
        slug: "hobi",
        parent_slug: None,
        image_url: "https://placehold.co/640x360/png?text=Kategori+Hobi",
    },
    CategorySeed {
        name: "Koleksi",
        slug: "koleksi",
        parent_slug: None,
        image_url: "https://placehold.co/640x360/png?text=Kategori+Koleksi",
    },
    CategorySeed {
        name: "Handphone",
        slug: "handphone",
        parent_slug: Some("elektronik"),
        image_url: "https://placehold.co/640x360/png?text=Subkategori+Handphone",
    },
    CategorySeed {
        name: "Laptop",
        slug: "laptop",
        parent_slug: Some("elektronik"),
        image_url: "https://placehold.co/640x360/png?text=Subkategori+Laptop",
    },
    CategorySeed {
        name: "Kamera",
        slug: "kamera",
        parent_slug: Some("elektronik"),
        image_url: "https://placehold.co/640x360/png?text=Subkategori+Kamera",
    },
    CategorySeed {
        name: "Pria",
        slug: "fashion-pria",
        parent_slug: Some("fashion"),
        image_url: "https://placehold.co/640x360/png?text=Subkategori+Fashion+Pria",
    },
    CategorySeed {
        name: "Wanita",
        slug: "fashion-wanita",
        parent_slug: Some("fashion"),
        image_url: "https://placehold.co/640x360/png?text=Subkategori+Fashion+Wanita",
    },
    CategorySeed {
        name: "Dapur",
        slug: "dapur",
        parent_slug: Some("rumah-tangga"),
        image_url: "https://placehold.co/640x360/png?text=Subkategori+Dapur",
    },
    CategorySeed {
        name: "Smart Home",
        slug: "smart-home",
        parent_slug: Some("rumah-tangga"),
        image_url: "https://placehold.co/640x360/png?text=Subkategori+Smart+Home",
    },
    CategorySeed {
        name: "Olahraga",
        slug: "olahraga",
        parent_slug: Some("hobi"),
        image_url: "https://placehold.co/640x360/png?text=Subkategori+Olahraga",
    },
    CategorySeed {
        name: "Gaming",
        slug: "gaming",
        parent_slug: Some("hobi"),
        image_url: "https://placehold.co/640x360/png?text=Subkategori+Gaming",
    },
    CategorySeed {
        name: "Jam Koleksi",
        slug: "jam-koleksi",
        parent_slug: Some("koleksi"),
        image_url: "https://placehold.co/640x360/png?text=Subkategori+Jam+Koleksi",
    },
];

const LISTINGS: &[ListingSeed] = &[
    ListingSeed {
        id: "6f4fa684-0266-4afb-b3ef-cf11742f4f01",
        seller_id: "a6bf5e42-c500-4299-8607-891cb9f4c501",
        seller_name: "Hakim Store",
        category_slug: "handphone",
        title: "iPhone 14 Pro 256GB Deep Purple",
        description: "Kondisi 97 persen, fullset dus dan kabel, battery health 89 persen.",
        start_price: 12000000,
        reserve_price: Some(13000000),
        current_price: 13450000,
        min_increment: 100000,
        bid_count: 14,
        starts_hours_ago: 36,
        ends_in_hours: 28,
        images: &[
            "https://placehold.co/1200x900/png?text=iPhone+14+Pro+Front",
            "https://placehold.co/1200x900/png?text=iPhone+14+Pro+Back",
            "https://placehold.co/1200x900/png?text=iPhone+14+Pro+Accessories",
        ],
    },
    ListingSeed {
        id: "6f4fa684-0266-4afb-b3ef-cf11742f4f02",
        seller_id: "a6bf5e42-c500-4299-8607-891cb9f4c501",
        seller_name: "Hakim Store",
        category_slug: "laptop",
        title: "MacBook Air M2 16GB 512GB Midnight",
        description: "Pemakaian ringan untuk coding, siklus baterai rendah, mulus tanpa dent.",
        start_price: 15000000,
        reserve_price: Some(16500000),
        current_price: 16900000,
        min_increment: 150000,
        bid_count: 11,
        starts_hours_ago: 40,
        ends_in_hours: 32,
        images: &[
            "https://placehold.co/1200x900/png?text=MacBook+Air+M2+Open",
            "https://placehold.co/1200x900/png?text=MacBook+Air+M2+Keyboard",
        ],
    },
    ListingSeed {
        id: "6f4fa684-0266-4afb-b3ef-cf11742f4f03",
        seller_id: "fc5ea6c0-2bb7-4d82-bf66-d95e6cc1f502",
        seller_name: "Nusa Camera",
        category_slug: "kamera",
        title: "Sony A7 IV Body Only",
        description: "Shutter count rendah, sensor bersih, cocok untuk foto dan video profesional.",
        start_price: 26000000,
        reserve_price: Some(27500000),
        current_price: 28100000,
        min_increment: 250000,
        bid_count: 8,
        starts_hours_ago: 30,
        ends_in_hours: 20,
        images: &[
            "https://placehold.co/1200x900/png?text=Sony+A7+IV+Front",
            "https://placehold.co/1200x900/png?text=Sony+A7+IV+Top",
            "https://placehold.co/1200x900/png?text=Sony+A7+IV+Sensor",
        ],
    },
    ListingSeed {
        id: "6f4fa684-0266-4afb-b3ef-cf11742f4f04",
        seller_id: "582d4da5-d50c-4c56-95bd-c971d8472503",
        seller_name: "Rama Outfit",
        category_slug: "fashion-pria",
        title: "Jaket Denim Levi's Vintage",
        description: "Original second, warna masih pekat, size L fit 70x56.",
        start_price: 850000,
        reserve_price: Some(1000000),
        current_price: 1125000,
        min_increment: 25000,
        bid_count: 19,
        starts_hours_ago: 16,
        ends_in_hours: 14,
        images: &[
            "https://placehold.co/1200x900/png?text=Levis+Denim+Jacket+Front",
            "https://placehold.co/1200x900/png?text=Levis+Denim+Jacket+Detail",
        ],
    },
    ListingSeed {
        id: "6f4fa684-0266-4afb-b3ef-cf11742f4f05",
        seller_id: "582d4da5-d50c-4c56-95bd-c971d8472503",
        seller_name: "Rama Outfit",
        category_slug: "fashion-wanita",
        title: "Tas Coach Tabby Shoulder Bag",
        description: "Kondisi sangat baik, hardware minim baret, include dustbag.",
        start_price: 3200000,
        reserve_price: Some(3600000),
        current_price: 3750000,
        min_increment: 50000,
        bid_count: 10,
        starts_hours_ago: 22,
        ends_in_hours: 26,
        images: &[
            "https://placehold.co/1200x900/png?text=Coach+Tabby+Bag+Front",
            "https://placehold.co/1200x900/png?text=Coach+Tabby+Bag+Inside",
        ],
    },
    ListingSeed {
        id: "6f4fa684-0266-4afb-b3ef-cf11742f4f06",
        seller_id: "0b29758c-fb0f-4d3a-ab7b-bd66868c4504",
        seller_name: "HomeLab",
        category_slug: "dapur",
        title: "Nespresso Essenza Mini",
        description: "Mesin kopi kapsul compact, heating normal, bekas pakai pribadi.",
        start_price: 1400000,
        reserve_price: Some(1700000),
        current_price: 1780000,
        min_increment: 25000,
        bid_count: 7,
        starts_hours_ago: 12,
        ends_in_hours: 30,
        images: &[
            "https://placehold.co/1200x900/png?text=Nespresso+Essenza+Mini+Front",
            "https://placehold.co/1200x900/png?text=Nespresso+Essenza+Mini+Set",
        ],
    },
    ListingSeed {
        id: "6f4fa684-0266-4afb-b3ef-cf11742f4f07",
        seller_id: "0b29758c-fb0f-4d3a-ab7b-bd66868c4504",
        seller_name: "HomeLab",
        category_slug: "smart-home",
        title: "Google Nest Hub Gen 2",
        description: "Layar mulus, speaker oke, siap dipakai untuk kontrol smart home.",
        start_price: 900000,
        reserve_price: Some(1100000),
        current_price: 1140000,
        min_increment: 20000,
        bid_count: 12,
        starts_hours_ago: 18,
        ends_in_hours: 40,
        images: &[
            "https://placehold.co/1200x900/png?text=Google+Nest+Hub+Display",
            "https://placehold.co/1200x900/png?text=Google+Nest+Hub+Back",
        ],
    },
    ListingSeed {
        id: "6f4fa684-0266-4afb-b3ef-cf11742f4f08",
        seller_id: "32abf8ea-8e24-4896-bf3e-62a8d6a4f505",
        seller_name: "Game Haven",
        category_slug: "gaming",
        title: "PlayStation 5 Disc Edition",
        description: "Unit Jepang, adaptor tersedia, controller original, suhu stabil.",
        start_price: 6200000,
        reserve_price: Some(6800000),
        current_price: 7010000,
        min_increment: 100000,
        bid_count: 15,
        starts_hours_ago: 20,
        ends_in_hours: 22,
        images: &[
            "https://placehold.co/1200x900/png?text=PS5+Disc+Edition+Set",
            "https://placehold.co/1200x900/png?text=PS5+Disc+Edition+Console",
            "https://placehold.co/1200x900/png?text=PS5+DualSense+Controller",
        ],
    },
    ListingSeed {
        id: "6f4fa684-0266-4afb-b3ef-cf11742f4f09",
        seller_id: "32abf8ea-8e24-4896-bf3e-62a8d6a4f505",
        seller_name: "Game Haven",
        category_slug: "gaming",
        title: "Nintendo Switch OLED White",
        description: "Pemakaian casual, joystick no drift, lengkap dock dan charger.",
        start_price: 3500000,
        reserve_price: Some(3900000),
        current_price: 4025000,
        min_increment: 50000,
        bid_count: 9,
        starts_hours_ago: 26,
        ends_in_hours: 44,
        images: &[
            "https://placehold.co/1200x900/png?text=Nintendo+Switch+OLED+Set",
            "https://placehold.co/1200x900/png?text=Nintendo+Switch+OLED+Dock",
        ],
    },
    ListingSeed {
        id: "6f4fa684-0266-4afb-b3ef-cf11742f4f0a",
        seller_id: "fc5ea6c0-2bb7-4d82-bf66-d95e6cc1f502",
        seller_name: "Nusa Camera",
        category_slug: "kamera",
        title: "Fujifilm X100V Silver",
        description: "Kamera street favorit, kondisi mulus, include lens hood dan battery ekstra.",
        start_price: 18000000,
        reserve_price: Some(19500000),
        current_price: 20100000,
        min_increment: 200000,
        bid_count: 13,
        starts_hours_ago: 28,
        ends_in_hours: 25,
        images: &[
            "https://placehold.co/1200x900/png?text=Fujifilm+X100V+Front",
            "https://placehold.co/1200x900/png?text=Fujifilm+X100V+Top",
        ],
    },
    ListingSeed {
        id: "6f4fa684-0266-4afb-b3ef-cf11742f4f0b",
        seller_id: "a6bf5e42-c500-4299-8607-891cb9f4c501",
        seller_name: "Hakim Store",
        category_slug: "handphone",
        title: "Samsung Galaxy S24 Ultra 512GB",
        description: "Resmi Indonesia, S-Pen normal, body mulus, masih garansi toko.",
        start_price: 14500000,
        reserve_price: Some(15500000),
        current_price: 15950000,
        min_increment: 100000,
        bid_count: 6,
        starts_hours_ago: 10,
        ends_in_hours: 52,
        images: &[
            "https://placehold.co/1200x900/png?text=Galaxy+S24+Ultra+Front",
            "https://placehold.co/1200x900/png?text=Galaxy+S24+Ultra+Spen",
        ],
    },
    ListingSeed {
        id: "6f4fa684-0266-4afb-b3ef-cf11742f4f0c",
        seller_id: "0b29758c-fb0f-4d3a-ab7b-bd66868c4504",
        seller_name: "HomeLab",
        category_slug: "olahraga",
        title: "Sepeda Lipat Brompton M6L",
        description: "Frame lurus, groupset terawat, cocok untuk commuting harian.",
        start_price: 21000000,
        reserve_price: Some(22500000),
        current_price: 23250000,
        min_increment: 250000,
        bid_count: 5,
        starts_hours_ago: 14,
        ends_in_hours: 60,
        images: &[
            "https://placehold.co/1200x900/png?text=Brompton+M6L+Side",
            "https://placehold.co/1200x900/png?text=Brompton+M6L+Folded",
        ],
    },
    ListingSeed {
        id: "6f4fa684-0266-4afb-b3ef-cf11742f4f0d",
        seller_id: "582d4da5-d50c-4c56-95bd-c971d8472503",
        seller_name: "Rama Outfit",
        category_slug: "jam-koleksi",
        title: "Seiko Presage Cocktail Time SRPB41J1",
        description: "Dial sunburst biru, mesin automatic 4R35, kondisi rapi.",
        start_price: 4200000,
        reserve_price: Some(4700000),
        current_price: 4880000,
        min_increment: 50000,
        bid_count: 17,
        starts_hours_ago: 24,
        ends_in_hours: 18,
        images: &[
            "https://placehold.co/1200x900/png?text=Seiko+Presage+Dial",
            "https://placehold.co/1200x900/png?text=Seiko+Presage+Strap",
        ],
    },
    ListingSeed {
        id: "6f4fa684-0266-4afb-b3ef-cf11742f4f0e",
        seller_id: "32abf8ea-8e24-4896-bf3e-62a8d6a4f505",
        seller_name: "Game Haven",
        category_slug: "laptop",
        title: "ASUS ROG Zephyrus G14 RTX 4060",
        description: "Gaming laptop tipis, suhu aman, upgrade RAM 32GB.",
        start_price: 18500000,
        reserve_price: Some(20000000),
        current_price: 20600000,
        min_increment: 200000,
        bid_count: 9,
        starts_hours_ago: 34,
        ends_in_hours: 36,
        images: &[
            "https://placehold.co/1200x900/png?text=ROG+Zephyrus+G14+Open",
            "https://placehold.co/1200x900/png?text=ROG+Zephyrus+G14+Keyboard",
        ],
    },
    ListingSeed {
        id: "6f4fa684-0266-4afb-b3ef-cf11742f4f0f",
        seller_id: "fc5ea6c0-2bb7-4d82-bf66-d95e6cc1f502",
        seller_name: "Nusa Camera",
        category_slug: "kamera",
        title: "Canon RF 24-70mm f/2.8L IS USM",
        description: "Lensa tajam, stabilizer normal, kaca bebas jamur.",
        start_price: 21000000,
        reserve_price: Some(22500000),
        current_price: 22900000,
        min_increment: 200000,
        bid_count: 4,
        starts_hours_ago: 8,
        ends_in_hours: 66,
        images: &[
            "https://placehold.co/1200x900/png?text=Canon+RF+24-70+Lens",
            "https://placehold.co/1200x900/png?text=Canon+RF+24-70+Glass",
        ],
    },
    ListingSeed {
        id: "6f4fa684-0266-4afb-b3ef-cf11742f4f10",
        seller_id: "a6bf5e42-c500-4299-8607-891cb9f4c501",
        seller_name: "Hakim Store",
        category_slug: "smart-home",
        title: "Dyson Purifier Cool TP07",
        description: "Filter masih tebal, mode auto jalan, remote lengkap.",
        start_price: 5400000,
        reserve_price: Some(6000000),
        current_price: 6180000,
        min_increment: 75000,
        bid_count: 7,
        starts_hours_ago: 12,
        ends_in_hours: 48,
        images: &[
            "https://placehold.co/1200x900/png?text=Dyson+TP07+Front",
            "https://placehold.co/1200x900/png?text=Dyson+TP07+Remote",
        ],
    },
];

const ADMIN_USER_ID: &str = "11111111-1111-4111-8111-111111111111";
const SELLER_HAKIM_ID: &str = "a6bf5e42-c500-4299-8607-891cb9f4c501";
const SELLER_NUSA_ID: &str = "fc5ea6c0-2bb7-4d82-bf66-d95e6cc1f502";
const SELLER_RAMA_ID: &str = "582d4da5-d50c-4c56-95bd-c971d8472503";
const SELLER_HOMELAB_ID: &str = "0b29758c-fb0f-4d3a-ab7b-bd66868c4504";
const SELLER_GAMEHAVEN_ID: &str = "32abf8ea-8e24-4896-bf3e-62a8d6a4f505";

const BUYER_AYU_ID: &str = "9d7f2c42-6b44-4a6a-92fe-1880d7b1a601";
const BUYER_BIMO_ID: &str = "8f3de4a9-ff1c-43b1-9f74-e527f66da602";
const BUYER_CITRA_ID: &str = "7ab4db3f-6ba2-41f4-91f5-b69e6e1df603";

const BUYER_AYU_NAME: &str = "Ayu Pratama";
const BUYER_BIMO_NAME: &str = "Bimo Santoso";
const BUYER_CITRA_NAME: &str = "Citra Anggraini";

struct WalletSeed {
    user_id: &'static str,
    balance: i64,
    held_balance: i64,
}

struct AuctionSeed {
    id: &'static str,
    listing_id: &'static str,
    seller_id: &'static str,
    seller_name: &'static str,
    title: &'static str,
    description: &'static str,
    image_url: &'static str,
    start_price: i64,
    current_price: i64,
    reserve_price: Option<i64>,
    bid_increment: i64,
    bid_count: i32,
    status: &'static str,
    winner_id: Option<&'static str>,
    winner_name: Option<&'static str>,
    starts_offset_hours: i64,
    ends_offset_hours: i64,
    original_ends_offset_hours: i64,
    extension_count: i32,
    listing_status: &'static str,
}

struct BidSeed {
    id: &'static str,
    auction_id: &'static str,
    bidder_id: &'static str,
    bidder_name: &'static str,
    amount: i64,
    is_proxy: bool,
    is_winning: bool,
    created_offset_minutes: i64,
}

struct ProxyBidSeed {
    id: &'static str,
    auction_id: &'static str,
    bidder_id: &'static str,
    max_amount: i64,
    is_active: bool,
    created_offset_minutes: i64,
}

struct OrderSeed {
    id: &'static str,
    auction_id: &'static str,
    listing_id: &'static str,
    buyer_id: &'static str,
    buyer_name: &'static str,
    seller_id: &'static str,
    seller_name: &'static str,
    title: &'static str,
    image_url: &'static str,
    final_price: i64,
    status: &'static str,
    shipping_status: Option<&'static str>,
    carrier: Option<&'static str>,
    tracking_number: Option<&'static str>,
    estimated_delivery_offset_hours: Option<i64>,
    paid_offset_hours: Option<i64>,
    shipped_offset_hours: Option<i64>,
    delivered_offset_hours: Option<i64>,
    confirmed_offset_hours: Option<i64>,
    is_disputed: bool,
    created_offset_hours: i64,
    updated_offset_hours: i64,
}

struct DisputeSeed {
    id: &'static str,
    order_id: &'static str,
    opened_by: &'static str,
    reason: &'static str,
    description: &'static str,
    status: &'static str,
    resolution: Option<&'static str>,
    created_offset_hours: i64,
    resolved_offset_hours: Option<i64>,
}

struct WalletTransactionSeed {
    id: &'static str,
    wallet_id: &'static str,
    tx_type: &'static str,
    status: &'static str,
    amount: i64,
    balance_after: i64,
    reference_id: Option<&'static str>,
    reference_type: Option<&'static str>,
    description: &'static str,
    created_offset_hours: i64,
    completed_offset_hours: Option<i64>,
}

struct NotificationSeed {
    id: &'static str,
    user_id: &'static str,
    notification_type: &'static str,
    title: &'static str,
    message: &'static str,
    is_read: bool,
    reference_id: Option<&'static str>,
    reference_type: Option<&'static str>,
    created_offset_hours: i64,
    read_offset_hours: Option<i64>,
}

const WALLETS: &[WalletSeed] = &[
    WalletSeed {
        user_id: ADMIN_USER_ID,
        balance: 100_000_000,
        held_balance: 0,
    },
    WalletSeed {
        user_id: SELLER_HAKIM_ID,
        balance: 42_000_000,
        held_balance: 0,
    },
    WalletSeed {
        user_id: SELLER_NUSA_ID,
        balance: 96_200_000,
        held_balance: 0,
    },
    WalletSeed {
        user_id: SELLER_RAMA_ID,
        balance: 38_000_000,
        held_balance: 0,
    },
    WalletSeed {
        user_id: SELLER_HOMELAB_ID,
        balance: 46_780_000,
        held_balance: 0,
    },
    WalletSeed {
        user_id: SELLER_GAMEHAVEN_ID,
        balance: 40_000_000,
        held_balance: 0,
    },
    WalletSeed {
        user_id: BUYER_AYU_ID,
        balance: 65_000_000,
        held_balance: 13_450_000,
    },
    WalletSeed {
        user_id: BUYER_BIMO_ID,
        balance: 55_000_000,
        held_balance: 3_750_000,
    },
    WalletSeed {
        user_id: BUYER_CITRA_ID,
        balance: 60_000_000,
        held_balance: 7_010_000,
    },
];

const AUCTIONS: &[AuctionSeed] = &[
    AuctionSeed {
        id: "8a1fa684-0266-4afb-b3ef-cf11742fa001",
        listing_id: "6f4fa684-0266-4afb-b3ef-cf11742f4f01",
        seller_id: SELLER_HAKIM_ID,
        seller_name: "Hakim Store",
        title: "iPhone 14 Pro 256GB Deep Purple",
        description: "Kondisi 97 persen, fullset dus dan kabel, battery health 89 persen.",
        image_url: "https://placehold.co/1200x900/png?text=iPhone+14+Pro+Front",
        start_price: 12_000_000,
        current_price: 13_450_000,
        reserve_price: Some(13_000_000),
        bid_increment: 100_000,
        bid_count: 4,
        status: "ACTIVE",
        winner_id: Some(BUYER_AYU_ID),
        winner_name: Some(BUYER_AYU_NAME),
        starts_offset_hours: -36,
        ends_offset_hours: 28,
        original_ends_offset_hours: 28,
        extension_count: 0,
        listing_status: "ACTIVE",
    },
    AuctionSeed {
        id: "8a1fa684-0266-4afb-b3ef-cf11742fa002",
        listing_id: "6f4fa684-0266-4afb-b3ef-cf11742f4f02",
        seller_id: SELLER_HAKIM_ID,
        seller_name: "Hakim Store",
        title: "MacBook Air M2 16GB 512GB Midnight",
        description: "Pemakaian ringan untuk coding, siklus baterai rendah, mulus tanpa dent.",
        image_url: "https://placehold.co/1200x900/png?text=MacBook+Air+M2+Open",
        start_price: 15_000_000,
        current_price: 15_000_000,
        reserve_price: Some(16_500_000),
        bid_increment: 150_000,
        bid_count: 0,
        status: "SCHEDULED",
        winner_id: None,
        winner_name: None,
        starts_offset_hours: 3,
        ends_offset_hours: 39,
        original_ends_offset_hours: 39,
        extension_count: 0,
        listing_status: "ACTIVE",
    },
    AuctionSeed {
        id: "8a1fa684-0266-4afb-b3ef-cf11742fa003",
        listing_id: "6f4fa684-0266-4afb-b3ef-cf11742f4f03",
        seller_id: SELLER_NUSA_ID,
        seller_name: "Nusa Camera",
        title: "Sony A7 IV Body Only",
        description: "Shutter count rendah, sensor bersih, cocok untuk foto dan video profesional.",
        image_url: "https://placehold.co/1200x900/png?text=Sony+A7+IV+Front",
        start_price: 26_000_000,
        current_price: 28_100_000,
        reserve_price: Some(27_500_000),
        bid_increment: 250_000,
        bid_count: 2,
        status: "WON",
        winner_id: Some(BUYER_AYU_ID),
        winner_name: Some(BUYER_AYU_NAME),
        starts_offset_hours: -72,
        ends_offset_hours: -12,
        original_ends_offset_hours: -12,
        extension_count: 0,
        listing_status: "SOLD",
    },
    AuctionSeed {
        id: "8a1fa684-0266-4afb-b3ef-cf11742fa004",
        listing_id: "6f4fa684-0266-4afb-b3ef-cf11742f4f04",
        seller_id: SELLER_RAMA_ID,
        seller_name: "Rama Outfit",
        title: "Jaket Denim Levi's Vintage",
        description: "Original second, warna masih pekat, size L fit 70x56.",
        image_url: "https://placehold.co/1200x900/png?text=Levis+Denim+Jacket+Front",
        start_price: 850_000,
        current_price: 980_000,
        reserve_price: Some(1_000_000),
        bid_increment: 25_000,
        bid_count: 1,
        status: "UNSOLD",
        winner_id: None,
        winner_name: None,
        starts_offset_hours: -26,
        ends_offset_hours: -2,
        original_ends_offset_hours: -2,
        extension_count: 0,
        listing_status: "EXPIRED",
    },
    AuctionSeed {
        id: "8a1fa684-0266-4afb-b3ef-cf11742fa005",
        listing_id: "6f4fa684-0266-4afb-b3ef-cf11742f4f05",
        seller_id: SELLER_RAMA_ID,
        seller_name: "Rama Outfit",
        title: "Tas Coach Tabby Shoulder Bag",
        description: "Kondisi sangat baik, hardware minim baret, include dustbag.",
        image_url: "https://placehold.co/1200x900/png?text=Coach+Tabby+Bag+Front",
        start_price: 3_200_000,
        current_price: 3_750_000,
        reserve_price: Some(3_600_000),
        bid_increment: 50_000,
        bid_count: 2,
        status: "EXTENDED",
        winner_id: Some(BUYER_BIMO_ID),
        winner_name: Some(BUYER_BIMO_NAME),
        starts_offset_hours: -22,
        ends_offset_hours: 26,
        original_ends_offset_hours: 20,
        extension_count: 1,
        listing_status: "ACTIVE",
    },
    AuctionSeed {
        id: "8a1fa684-0266-4afb-b3ef-cf11742fa006",
        listing_id: "6f4fa684-0266-4afb-b3ef-cf11742f4f06",
        seller_id: SELLER_HOMELAB_ID,
        seller_name: "HomeLab",
        title: "Nespresso Essenza Mini",
        description: "Mesin kopi kapsul compact, heating normal, bekas pakai pribadi.",
        image_url: "https://placehold.co/1200x900/png?text=Nespresso+Essenza+Mini+Front",
        start_price: 1_400_000,
        current_price: 1_780_000,
        reserve_price: Some(1_700_000),
        bid_increment: 25_000,
        bid_count: 2,
        status: "WON",
        winner_id: Some(BUYER_BIMO_ID),
        winner_name: Some(BUYER_BIMO_NAME),
        starts_offset_hours: -40,
        ends_offset_hours: -6,
        original_ends_offset_hours: -6,
        extension_count: 0,
        listing_status: "SOLD",
    },
    AuctionSeed {
        id: "8a1fa684-0266-4afb-b3ef-cf11742fa007",
        listing_id: "6f4fa684-0266-4afb-b3ef-cf11742f4f0a",
        seller_id: SELLER_NUSA_ID,
        seller_name: "Nusa Camera",
        title: "Fujifilm X100V Silver",
        description: "Kamera street favorit, kondisi mulus, include lens hood dan battery ekstra.",
        image_url: "https://placehold.co/1200x900/png?text=Fujifilm+X100V+Front",
        start_price: 18_000_000,
        current_price: 20_100_000,
        reserve_price: Some(19_500_000),
        bid_increment: 200_000,
        bid_count: 1,
        status: "WON",
        winner_id: Some(BUYER_CITRA_ID),
        winner_name: Some(BUYER_CITRA_NAME),
        starts_offset_hours: -30,
        ends_offset_hours: -4,
        original_ends_offset_hours: -4,
        extension_count: 0,
        listing_status: "SOLD",
    },
    AuctionSeed {
        id: "8a1fa684-0266-4afb-b3ef-cf11742fa008",
        listing_id: "6f4fa684-0266-4afb-b3ef-cf11742f4f08",
        seller_id: SELLER_GAMEHAVEN_ID,
        seller_name: "Game Haven",
        title: "PlayStation 5 Disc Edition",
        description: "Unit Jepang, adaptor tersedia, controller original, suhu stabil.",
        image_url: "https://placehold.co/1200x900/png?text=PS5+Disc+Edition+Set",
        start_price: 6_200_000,
        current_price: 7_010_000,
        reserve_price: Some(6_800_000),
        bid_increment: 100_000,
        bid_count: 2,
        status: "ACTIVE",
        winner_id: Some(BUYER_CITRA_ID),
        winner_name: Some(BUYER_CITRA_NAME),
        starts_offset_hours: -20,
        ends_offset_hours: 22,
        original_ends_offset_hours: 22,
        extension_count: 0,
        listing_status: "ACTIVE",
    },
];

const BIDS: &[BidSeed] = &[
    BidSeed {
        id: "9b1fa684-0266-4afb-b3ef-cf11742fb001",
        auction_id: "8a1fa684-0266-4afb-b3ef-cf11742fa001",
        bidder_id: BUYER_BIMO_ID,
        bidder_name: BUYER_BIMO_NAME,
        amount: 12_300_000,
        is_proxy: false,
        is_winning: false,
        created_offset_minutes: -180,
    },
    BidSeed {
        id: "9b1fa684-0266-4afb-b3ef-cf11742fb002",
        auction_id: "8a1fa684-0266-4afb-b3ef-cf11742fa001",
        bidder_id: BUYER_AYU_ID,
        bidder_name: BUYER_AYU_NAME,
        amount: 12_800_000,
        is_proxy: false,
        is_winning: false,
        created_offset_minutes: -150,
    },
    BidSeed {
        id: "9b1fa684-0266-4afb-b3ef-cf11742fb003",
        auction_id: "8a1fa684-0266-4afb-b3ef-cf11742fa001",
        bidder_id: BUYER_CITRA_ID,
        bidder_name: BUYER_CITRA_NAME,
        amount: 13_100_000,
        is_proxy: false,
        is_winning: false,
        created_offset_minutes: -120,
    },
    BidSeed {
        id: "9b1fa684-0266-4afb-b3ef-cf11742fb004",
        auction_id: "8a1fa684-0266-4afb-b3ef-cf11742fa001",
        bidder_id: BUYER_AYU_ID,
        bidder_name: BUYER_AYU_NAME,
        amount: 13_450_000,
        is_proxy: false,
        is_winning: true,
        created_offset_minutes: -90,
    },
    BidSeed {
        id: "9b1fa684-0266-4afb-b3ef-cf11742fb005",
        auction_id: "8a1fa684-0266-4afb-b3ef-cf11742fa003",
        bidder_id: BUYER_CITRA_ID,
        bidder_name: BUYER_CITRA_NAME,
        amount: 27_200_000,
        is_proxy: false,
        is_winning: false,
        created_offset_minutes: -1600,
    },
    BidSeed {
        id: "9b1fa684-0266-4afb-b3ef-cf11742fb006",
        auction_id: "8a1fa684-0266-4afb-b3ef-cf11742fa003",
        bidder_id: BUYER_AYU_ID,
        bidder_name: BUYER_AYU_NAME,
        amount: 28_100_000,
        is_proxy: false,
        is_winning: true,
        created_offset_minutes: -1500,
    },
    BidSeed {
        id: "9b1fa684-0266-4afb-b3ef-cf11742fb007",
        auction_id: "8a1fa684-0266-4afb-b3ef-cf11742fa004",
        bidder_id: BUYER_BIMO_ID,
        bidder_name: BUYER_BIMO_NAME,
        amount: 980_000,
        is_proxy: false,
        is_winning: false,
        created_offset_minutes: -240,
    },
    BidSeed {
        id: "9b1fa684-0266-4afb-b3ef-cf11742fb008",
        auction_id: "8a1fa684-0266-4afb-b3ef-cf11742fa005",
        bidder_id: BUYER_AYU_ID,
        bidder_name: BUYER_AYU_NAME,
        amount: 3_400_000,
        is_proxy: false,
        is_winning: false,
        created_offset_minutes: -300,
    },
    BidSeed {
        id: "9b1fa684-0266-4afb-b3ef-cf11742fb009",
        auction_id: "8a1fa684-0266-4afb-b3ef-cf11742fa005",
        bidder_id: BUYER_BIMO_ID,
        bidder_name: BUYER_BIMO_NAME,
        amount: 3_750_000,
        is_proxy: false,
        is_winning: true,
        created_offset_minutes: -260,
    },
    BidSeed {
        id: "9b1fa684-0266-4afb-b3ef-cf11742fb00a",
        auction_id: "8a1fa684-0266-4afb-b3ef-cf11742fa006",
        bidder_id: BUYER_AYU_ID,
        bidder_name: BUYER_AYU_NAME,
        amount: 1_720_000,
        is_proxy: false,
        is_winning: false,
        created_offset_minutes: -2200,
    },
    BidSeed {
        id: "9b1fa684-0266-4afb-b3ef-cf11742fb00b",
        auction_id: "8a1fa684-0266-4afb-b3ef-cf11742fa006",
        bidder_id: BUYER_BIMO_ID,
        bidder_name: BUYER_BIMO_NAME,
        amount: 1_780_000,
        is_proxy: false,
        is_winning: true,
        created_offset_minutes: -2100,
    },
    BidSeed {
        id: "9b1fa684-0266-4afb-b3ef-cf11742fb00c",
        auction_id: "8a1fa684-0266-4afb-b3ef-cf11742fa007",
        bidder_id: BUYER_CITRA_ID,
        bidder_name: BUYER_CITRA_NAME,
        amount: 20_100_000,
        is_proxy: false,
        is_winning: true,
        created_offset_minutes: -1500,
    },
    BidSeed {
        id: "9b1fa684-0266-4afb-b3ef-cf11742fb00d",
        auction_id: "8a1fa684-0266-4afb-b3ef-cf11742fa008",
        bidder_id: BUYER_BIMO_ID,
        bidder_name: BUYER_BIMO_NAME,
        amount: 6_400_000,
        is_proxy: false,
        is_winning: false,
        created_offset_minutes: -200,
    },
    BidSeed {
        id: "9b1fa684-0266-4afb-b3ef-cf11742fb00e",
        auction_id: "8a1fa684-0266-4afb-b3ef-cf11742fa008",
        bidder_id: BUYER_CITRA_ID,
        bidder_name: BUYER_CITRA_NAME,
        amount: 7_010_000,
        is_proxy: false,
        is_winning: true,
        created_offset_minutes: -150,
    },
];

const PROXY_BIDS: &[ProxyBidSeed] = &[
    ProxyBidSeed {
        id: "ab1fa684-0266-4afb-b3ef-cf11742fc001",
        auction_id: "8a1fa684-0266-4afb-b3ef-cf11742fa001",
        bidder_id: BUYER_CITRA_ID,
        max_amount: 14_000_000,
        is_active: true,
        created_offset_minutes: -130,
    },
    ProxyBidSeed {
        id: "ab1fa684-0266-4afb-b3ef-cf11742fc002",
        auction_id: "8a1fa684-0266-4afb-b3ef-cf11742fa005",
        bidder_id: BUYER_BIMO_ID,
        max_amount: 4_000_000,
        is_active: true,
        created_offset_minutes: -250,
    },
    ProxyBidSeed {
        id: "ab1fa684-0266-4afb-b3ef-cf11742fc003",
        auction_id: "8a1fa684-0266-4afb-b3ef-cf11742fa008",
        bidder_id: BUYER_CITRA_ID,
        max_amount: 7_300_000,
        is_active: true,
        created_offset_minutes: -140,
    },
];

const ORDERS: &[OrderSeed] = &[
    OrderSeed {
        id: "c91fa684-0266-4afb-b3ef-cf11742fd001",
        auction_id: "8a1fa684-0266-4afb-b3ef-cf11742fa003",
        listing_id: "6f4fa684-0266-4afb-b3ef-cf11742f4f03",
        buyer_id: BUYER_AYU_ID,
        buyer_name: BUYER_AYU_NAME,
        seller_id: SELLER_NUSA_ID,
        seller_name: "Nusa Camera",
        title: "Sony A7 IV Body Only",
        image_url: "https://placehold.co/1200x900/png?text=Sony+A7+IV+Front",
        final_price: 28_100_000,
        status: "DISPUTED",
        shipping_status: Some("DELIVERED"),
        carrier: Some("JNE"),
        tracking_number: Some("JNE-SONY-28100000"),
        estimated_delivery_offset_hours: Some(-14),
        paid_offset_hours: Some(-22),
        shipped_offset_hours: Some(-20),
        delivered_offset_hours: Some(-14),
        confirmed_offset_hours: None,
        is_disputed: true,
        created_offset_hours: -24,
        updated_offset_hours: -8,
    },
    OrderSeed {
        id: "c91fa684-0266-4afb-b3ef-cf11742fd002",
        auction_id: "8a1fa684-0266-4afb-b3ef-cf11742fa006",
        listing_id: "6f4fa684-0266-4afb-b3ef-cf11742f4f06",
        buyer_id: BUYER_BIMO_ID,
        buyer_name: BUYER_BIMO_NAME,
        seller_id: SELLER_HOMELAB_ID,
        seller_name: "HomeLab",
        title: "Nespresso Essenza Mini",
        image_url: "https://placehold.co/1200x900/png?text=Nespresso+Essenza+Mini+Front",
        final_price: 1_780_000,
        status: "CONFIRMED",
        shipping_status: Some("DELIVERED"),
        carrier: Some("SiCepat"),
        tracking_number: Some("SICEPAT-NESP-1780000"),
        estimated_delivery_offset_hours: Some(-24),
        paid_offset_hours: Some(-30),
        shipped_offset_hours: Some(-28),
        delivered_offset_hours: Some(-24),
        confirmed_offset_hours: Some(-20),
        is_disputed: false,
        created_offset_hours: -32,
        updated_offset_hours: -20,
    },
    OrderSeed {
        id: "c91fa684-0266-4afb-b3ef-cf11742fd003",
        auction_id: "8a1fa684-0266-4afb-b3ef-cf11742fa007",
        listing_id: "6f4fa684-0266-4afb-b3ef-cf11742f4f0a",
        buyer_id: BUYER_CITRA_ID,
        buyer_name: BUYER_CITRA_NAME,
        seller_id: SELLER_NUSA_ID,
        seller_name: "Nusa Camera",
        title: "Fujifilm X100V Silver",
        image_url: "https://placehold.co/1200x900/png?text=Fujifilm+X100V+Front",
        final_price: 20_100_000,
        status: "REFUNDED",
        shipping_status: Some("DELIVERED"),
        carrier: Some("AnterAja"),
        tracking_number: Some("ANTERAJA-X100V-20100000"),
        estimated_delivery_offset_hours: Some(-20),
        paid_offset_hours: Some(-26),
        shipped_offset_hours: Some(-24),
        delivered_offset_hours: Some(-20),
        confirmed_offset_hours: None,
        is_disputed: true,
        created_offset_hours: -30,
        updated_offset_hours: -2,
    },
];

const DISPUTES: &[DisputeSeed] = &[
    DisputeSeed {
        id: "d71fa684-0266-4afb-b3ef-cf11742fe001",
        order_id: "c91fa684-0266-4afb-b3ef-cf11742fd001",
        opened_by: BUYER_AYU_ID,
        reason: "ITEM_NOT_AS_DESCRIBED",
        description: "Produk diterima tetapi shutter count tidak sesuai deskripsi listing.",
        status: "OPEN",
        resolution: None,
        created_offset_hours: -7,
        resolved_offset_hours: None,
    },
    DisputeSeed {
        id: "d71fa684-0266-4afb-b3ef-cf11742fe002",
        order_id: "c91fa684-0266-4afb-b3ef-cf11742fd003",
        opened_by: BUYER_CITRA_ID,
        reason: "ITEM_NOT_RECEIVED",
        description: "Barang sempat tertahan lama dan buyer meminta penyelesaian refund.",
        status: "RESOLVED_BUYER",
        resolution: Some("Refund penuh disetujui admin karena bukti pengiriman tidak memadai."),
        created_offset_hours: -18,
        resolved_offset_hours: Some(-2),
    },
];

const WALLET_TRANSACTIONS: &[WalletTransactionSeed] = &[
    WalletTransactionSeed {
        id: "e61fa684-0266-4afb-b3ef-cf11742ff001",
        wallet_id: ADMIN_USER_ID,
        tx_type: "TOPUP",
        status: "COMPLETED",
        amount: 100_000_000,
        balance_after: 100_000_000,
        reference_id: None,
        reference_type: None,
        description: "Initial seed topup",
        created_offset_hours: -300,
        completed_offset_hours: Some(-300),
    },
    WalletTransactionSeed {
        id: "e61fa684-0266-4afb-b3ef-cf11742ff002",
        wallet_id: SELLER_HAKIM_ID,
        tx_type: "TOPUP",
        status: "COMPLETED",
        amount: 42_000_000,
        balance_after: 42_000_000,
        reference_id: None,
        reference_type: None,
        description: "Initial seed topup",
        created_offset_hours: -300,
        completed_offset_hours: Some(-300),
    },
    WalletTransactionSeed {
        id: "e61fa684-0266-4afb-b3ef-cf11742ff003",
        wallet_id: SELLER_NUSA_ID,
        tx_type: "TOPUP",
        status: "COMPLETED",
        amount: 48_000_000,
        balance_after: 48_000_000,
        reference_id: None,
        reference_type: None,
        description: "Initial seed topup",
        created_offset_hours: -300,
        completed_offset_hours: Some(-300),
    },
    WalletTransactionSeed {
        id: "e61fa684-0266-4afb-b3ef-cf11742ff004",
        wallet_id: SELLER_RAMA_ID,
        tx_type: "TOPUP",
        status: "COMPLETED",
        amount: 38_000_000,
        balance_after: 38_000_000,
        reference_id: None,
        reference_type: None,
        description: "Initial seed topup",
        created_offset_hours: -300,
        completed_offset_hours: Some(-300),
    },
    WalletTransactionSeed {
        id: "e61fa684-0266-4afb-b3ef-cf11742ff005",
        wallet_id: SELLER_HOMELAB_ID,
        tx_type: "TOPUP",
        status: "COMPLETED",
        amount: 45_000_000,
        balance_after: 45_000_000,
        reference_id: None,
        reference_type: None,
        description: "Initial seed topup",
        created_offset_hours: -300,
        completed_offset_hours: Some(-300),
    },
    WalletTransactionSeed {
        id: "e61fa684-0266-4afb-b3ef-cf11742ff006",
        wallet_id: SELLER_GAMEHAVEN_ID,
        tx_type: "TOPUP",
        status: "COMPLETED",
        amount: 40_000_000,
        balance_after: 40_000_000,
        reference_id: None,
        reference_type: None,
        description: "Initial seed topup",
        created_offset_hours: -300,
        completed_offset_hours: Some(-300),
    },
    WalletTransactionSeed {
        id: "e61fa684-0266-4afb-b3ef-cf11742ff007",
        wallet_id: BUYER_AYU_ID,
        tx_type: "TOPUP",
        status: "COMPLETED",
        amount: 65_000_000,
        balance_after: 65_000_000,
        reference_id: None,
        reference_type: None,
        description: "Initial seed topup",
        created_offset_hours: -300,
        completed_offset_hours: Some(-300),
    },
    WalletTransactionSeed {
        id: "e61fa684-0266-4afb-b3ef-cf11742ff008",
        wallet_id: BUYER_BIMO_ID,
        tx_type: "TOPUP",
        status: "COMPLETED",
        amount: 55_000_000,
        balance_after: 55_000_000,
        reference_id: None,
        reference_type: None,
        description: "Initial seed topup",
        created_offset_hours: -300,
        completed_offset_hours: Some(-300),
    },
    WalletTransactionSeed {
        id: "e61fa684-0266-4afb-b3ef-cf11742ff009",
        wallet_id: BUYER_CITRA_ID,
        tx_type: "TOPUP",
        status: "COMPLETED",
        amount: 60_000_000,
        balance_after: 60_000_000,
        reference_id: None,
        reference_type: None,
        description: "Initial seed topup",
        created_offset_hours: -300,
        completed_offset_hours: Some(-300),
    },
    WalletTransactionSeed {
        id: "e61fa684-0266-4afb-b3ef-cf11742ff00a",
        wallet_id: BUYER_AYU_ID,
        tx_type: "BID_HOLD",
        status: "COMPLETED",
        amount: 13_450_000,
        balance_after: 65_000_000,
        reference_id: Some("8a1fa684-0266-4afb-b3ef-cf11742fa001"),
        reference_type: Some("auction"),
        description: "Bid hold for active auction",
        created_offset_hours: -5,
        completed_offset_hours: Some(-5),
    },
    WalletTransactionSeed {
        id: "e61fa684-0266-4afb-b3ef-cf11742ff00b",
        wallet_id: BUYER_BIMO_ID,
        tx_type: "BID_HOLD",
        status: "COMPLETED",
        amount: 3_750_000,
        balance_after: 55_000_000,
        reference_id: Some("8a1fa684-0266-4afb-b3ef-cf11742fa005"),
        reference_type: Some("auction"),
        description: "Bid hold for extended auction",
        created_offset_hours: -5,
        completed_offset_hours: Some(-5),
    },
    WalletTransactionSeed {
        id: "e61fa684-0266-4afb-b3ef-cf11742ff00c",
        wallet_id: BUYER_CITRA_ID,
        tx_type: "BID_HOLD",
        status: "COMPLETED",
        amount: 7_010_000,
        balance_after: 60_000_000,
        reference_id: Some("8a1fa684-0266-4afb-b3ef-cf11742fa008"),
        reference_type: Some("auction"),
        description: "Bid hold for active auction",
        created_offset_hours: -4,
        completed_offset_hours: Some(-4),
    },
    WalletTransactionSeed {
        id: "e61fa684-0266-4afb-b3ef-cf11742ff00d",
        wallet_id: BUYER_AYU_ID,
        tx_type: "BID_CONVERT",
        status: "COMPLETED",
        amount: 28_100_000,
        balance_after: 36_900_000,
        reference_id: Some("8a1fa684-0266-4afb-b3ef-cf11742fa003"),
        reference_type: Some("auction"),
        description: "Bid converted to payment",
        created_offset_hours: -11,
        completed_offset_hours: Some(-11),
    },
    WalletTransactionSeed {
        id: "e61fa684-0266-4afb-b3ef-cf11742ff00e",
        wallet_id: SELLER_NUSA_ID,
        tx_type: "PAYMENT_RECEIVED",
        status: "COMPLETED",
        amount: 28_100_000,
        balance_after: 76_100_000,
        reference_id: Some("8a1fa684-0266-4afb-b3ef-cf11742fa003"),
        reference_type: Some("auction"),
        description: "Auction payment received",
        created_offset_hours: -11,
        completed_offset_hours: Some(-11),
    },
    WalletTransactionSeed {
        id: "e61fa684-0266-4afb-b3ef-cf11742ff00f",
        wallet_id: BUYER_BIMO_ID,
        tx_type: "BID_CONVERT",
        status: "COMPLETED",
        amount: 1_780_000,
        balance_after: 53_220_000,
        reference_id: Some("8a1fa684-0266-4afb-b3ef-cf11742fa006"),
        reference_type: Some("auction"),
        description: "Bid converted to payment",
        created_offset_hours: -6,
        completed_offset_hours: Some(-6),
    },
    WalletTransactionSeed {
        id: "e61fa684-0266-4afb-b3ef-cf11742ff010",
        wallet_id: SELLER_HOMELAB_ID,
        tx_type: "PAYMENT_RECEIVED",
        status: "COMPLETED",
        amount: 1_780_000,
        balance_after: 46_780_000,
        reference_id: Some("8a1fa684-0266-4afb-b3ef-cf11742fa006"),
        reference_type: Some("auction"),
        description: "Auction payment received",
        created_offset_hours: -6,
        completed_offset_hours: Some(-6),
    },
    WalletTransactionSeed {
        id: "e61fa684-0266-4afb-b3ef-cf11742ff011",
        wallet_id: BUYER_CITRA_ID,
        tx_type: "BID_CONVERT",
        status: "COMPLETED",
        amount: 20_100_000,
        balance_after: 39_900_000,
        reference_id: Some("8a1fa684-0266-4afb-b3ef-cf11742fa007"),
        reference_type: Some("auction"),
        description: "Bid converted to payment",
        created_offset_hours: -4,
        completed_offset_hours: Some(-4),
    },
    WalletTransactionSeed {
        id: "e61fa684-0266-4afb-b3ef-cf11742ff012",
        wallet_id: SELLER_NUSA_ID,
        tx_type: "PAYMENT_RECEIVED",
        status: "COMPLETED",
        amount: 20_100_000,
        balance_after: 96_200_000,
        reference_id: Some("8a1fa684-0266-4afb-b3ef-cf11742fa007"),
        reference_type: Some("auction"),
        description: "Auction payment received",
        created_offset_hours: -4,
        completed_offset_hours: Some(-4),
    },
    WalletTransactionSeed {
        id: "e61fa684-0266-4afb-b3ef-cf11742ff013",
        wallet_id: BUYER_CITRA_ID,
        tx_type: "REFUND",
        status: "COMPLETED",
        amount: 20_100_000,
        balance_after: 60_000_000,
        reference_id: Some("d71fa684-0266-4afb-b3ef-cf11742fe002"),
        reference_type: Some("dispute"),
        description: "Refund due to dispute resolution",
        created_offset_hours: -1,
        completed_offset_hours: Some(-1),
    },
];

const NOTIFICATIONS: &[NotificationSeed] = &[
    NotificationSeed {
        id: "f51fa684-0266-4afb-b3ef-cf11742ff001",
        user_id: BUYER_BIMO_ID,
        notification_type: "BID_OUTBID",
        title: "Bid kamu tersalip",
        message: "Peserta lain menawar lebih tinggi pada iPhone 14 Pro.",
        is_read: false,
        reference_id: Some("8a1fa684-0266-4afb-b3ef-cf11742fa001"),
        reference_type: Some("auction"),
        created_offset_hours: -3,
        read_offset_hours: None,
    },
    NotificationSeed {
        id: "f51fa684-0266-4afb-b3ef-cf11742ff002",
        user_id: BUYER_AYU_ID,
        notification_type: "AUCTION_WON",
        title: "Kamu memenangkan lelang",
        message: "Lelang Sony A7 IV telah selesai dan kamu menjadi pemenang.",
        is_read: true,
        reference_id: Some("8a1fa684-0266-4afb-b3ef-cf11742fa003"),
        reference_type: Some("auction"),
        created_offset_hours: -11,
        read_offset_hours: Some(-10),
    },
    NotificationSeed {
        id: "f51fa684-0266-4afb-b3ef-cf11742ff003",
        user_id: SELLER_NUSA_ID,
        notification_type: "PAYMENT_RECEIVED",
        title: "Pembayaran diterima",
        message: "Dana dari lelang Sony A7 IV telah masuk ke wallet kamu.",
        is_read: false,
        reference_id: Some("8a1fa684-0266-4afb-b3ef-cf11742fa003"),
        reference_type: Some("auction"),
        created_offset_hours: -10,
        read_offset_hours: None,
    },
    NotificationSeed {
        id: "f51fa684-0266-4afb-b3ef-cf11742ff004",
        user_id: SELLER_NUSA_ID,
        notification_type: "DISPUTE_OPENED",
        title: "Dispute baru dibuka",
        message: "Buyer membuka dispute untuk order Sony A7 IV.",
        is_read: false,
        reference_id: Some("d71fa684-0266-4afb-b3ef-cf11742fe001"),
        reference_type: Some("dispute"),
        created_offset_hours: -7,
        read_offset_hours: None,
    },
    NotificationSeed {
        id: "f51fa684-0266-4afb-b3ef-cf11742ff005",
        user_id: BUYER_CITRA_ID,
        notification_type: "DISPUTE_RESOLVED",
        title: "Dispute selesai",
        message: "Dispute untuk Fujifilm X100V telah diselesaikan dengan refund buyer.",
        is_read: true,
        reference_id: Some("d71fa684-0266-4afb-b3ef-cf11742fe002"),
        reference_type: Some("dispute"),
        created_offset_hours: -2,
        read_offset_hours: Some(-1),
    },
    NotificationSeed {
        id: "f51fa684-0266-4afb-b3ef-cf11742ff006",
        user_id: BUYER_BIMO_ID,
        notification_type: "ORDER_SHIPPED",
        title: "Pesanan dikirim",
        message: "Pesanan Nespresso Essenza Mini sedang dalam pengiriman.",
        is_read: true,
        reference_id: Some("c91fa684-0266-4afb-b3ef-cf11742fd002"),
        reference_type: Some("order"),
        created_offset_hours: -27,
        read_offset_hours: Some(-26),
    },
    NotificationSeed {
        id: "f51fa684-0266-4afb-b3ef-cf11742ff007",
        user_id: BUYER_BIMO_ID,
        notification_type: "ORDER_DELIVERED",
        title: "Pesanan tiba",
        message: "Pesanan Nespresso Essenza Mini telah diterima dan menunggu konfirmasi.",
        is_read: false,
        reference_id: Some("c91fa684-0266-4afb-b3ef-cf11742fd002"),
        reference_type: Some("order"),
        created_offset_hours: -24,
        read_offset_hours: None,
    },
    NotificationSeed {
        id: "f51fa684-0266-4afb-b3ef-cf11742ff008",
        user_id: ADMIN_USER_ID,
        notification_type: "SYSTEM",
        title: "Seed data siap",
        message: "Dataset staging untuk modul admin/core telah diperbarui.",
        is_read: false,
        reference_id: None,
        reference_type: None,
        created_offset_hours: -1,
        read_offset_hours: None,
    },
];

#[tokio::main]
async fn main() -> Result<()> {
    init_tracer();
    dotenv::dotenv().ok();

    let config = AppConfig::new().context("failed to load app config")?;
    let pool = create_pool(&config.database_url).await?;
    run_pending_migrations(&pool).await?;

    seed_catalog(&pool).await?;
    seed_wallets(&pool).await?;
    seed_auctions_and_bids(&pool).await?;
    seed_orders_and_disputes(&pool).await?;
    seed_wallet_transactions(&pool).await?;
    seed_notifications(&pool).await?;

    tracing::info!(
        categories = CATEGORIES.len(),
        listings = LISTINGS.len(),
        wallets = WALLETS.len(),
        auctions = AUCTIONS.len(),
        bids = BIDS.len(),
        orders = ORDERS.len(),
        disputes = DISPUTES.len(),
        notifications = NOTIFICATIONS.len(),
        "Core seed completed"
    );
    println!(
        "Core seed completed: {} categories, {} listings, {} wallets, {} auctions, {} bids, {} orders, {} disputes, {} notifications.",
        CATEGORIES.len(),
        LISTINGS.len(),
        WALLETS.len(),
        AUCTIONS.len(),
        BIDS.len(),
        ORDERS.len(),
        DISPUTES.len(),
        NOTIFICATIONS.len()
    );
    Ok(())
}

async fn seed_catalog(pool: &PgPool) -> Result<()> {
    let mut tx = pool.begin().await.context("failed to begin transaction")?;

    let mut category_ids = HashMap::<String, i32>::new();

    for category in CATEGORIES {
        let parent_id = category
            .parent_slug
            .and_then(|slug| category_ids.get(slug).copied());

        let row = sqlx::query!(
            r#"
            INSERT INTO categories (name, slug, parent_id, image_url)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (slug)
            DO UPDATE SET
                name = EXCLUDED.name,
                parent_id = EXCLUDED.parent_id,
                image_url = EXCLUDED.image_url,
                updated_at = NOW()
            RETURNING id
            "#,
            category.name,
            category.slug,
            parent_id,
            category.image_url
        )
        .fetch_one(&mut *tx)
        .await
        .with_context(|| format!("failed to upsert category slug={}", category.slug))?;

        category_ids.insert(category.slug.to_string(), row.id);
    }

    for listing in LISTINGS {
        let listing_id = parse_uuid(listing.id, "listing id")?;
        let seller_id = parse_uuid(listing.seller_id, "seller id")?;
        let category_id = *category_ids
            .get(listing.category_slug)
            .with_context(|| format!("unknown category slug: {}", listing.category_slug))?;
        let category_name = CATEGORIES
            .iter()
            .find(|category| category.slug == listing.category_slug)
            .map(|category| category.name)
            .with_context(|| format!("missing category name for slug={}", listing.category_slug))?;

        let starts_at = Utc::now() - Duration::hours(listing.starts_hours_ago);
        let ends_at = Utc::now() + Duration::hours(listing.ends_in_hours);

        sqlx::query!(
            r#"
            INSERT INTO listings (
                id, seller_id, seller_name, category_id, category_name,
                title, description, start_price, reserve_price, current_price,
                min_increment, bid_count, status, auction_id, starts_at, ends_at,
                created_at, updated_at
            )
            VALUES (
                $1, $2, $3, $4, $5,
                $6, $7, $8, $9, $10,
                $11, $12, 'ACTIVE'::listing_status, NULL, $13, $14,
                NOW(), NOW()
            )
            ON CONFLICT (id)
            DO UPDATE SET
                seller_id = EXCLUDED.seller_id,
                seller_name = EXCLUDED.seller_name,
                category_id = EXCLUDED.category_id,
                category_name = EXCLUDED.category_name,
                title = EXCLUDED.title,
                description = EXCLUDED.description,
                start_price = EXCLUDED.start_price,
                reserve_price = EXCLUDED.reserve_price,
                current_price = EXCLUDED.current_price,
                min_increment = EXCLUDED.min_increment,
                bid_count = EXCLUDED.bid_count,
                status = EXCLUDED.status,
                starts_at = EXCLUDED.starts_at,
                ends_at = EXCLUDED.ends_at,
                updated_at = NOW()
            "#,
            listing_id,
            seller_id,
            listing.seller_name,
            category_id,
            category_name,
            listing.title,
            listing.description,
            listing.start_price,
            listing.reserve_price,
            listing.current_price,
            listing.min_increment,
            listing.bid_count,
            starts_at,
            ends_at
        )
        .execute(&mut *tx)
        .await
        .with_context(|| format!("failed to upsert listing id={}", listing.id))?;

        sqlx::query!(
            "DELETE FROM listing_images WHERE listing_id = $1",
            listing_id
        )
        .execute(&mut *tx)
        .await
        .with_context(|| format!("failed to clear listing images for id={}", listing.id))?;

        for (order, image_url) in listing.images.iter().enumerate() {
            sqlx::query!(
                r#"
                INSERT INTO listing_images (listing_id, url, "order")
                VALUES ($1, $2, $3)
                "#,
                listing_id,
                image_url,
                order as i32
            )
            .execute(&mut *tx)
            .await
            .with_context(|| {
                format!(
                    "failed to insert image order={} for listing id={}",
                    order, listing.id
                )
            })?;
        }
    }

    // Sync child_count to reflect current hierarchy.
    sqlx::query!(
        r#"
        UPDATE categories
        SET child_count = 0, updated_at = NOW()
        "#
    )
    .execute(&mut *tx)
    .await
    .context("failed to reset category child_count")?;

    sqlx::query!(
        r#"
        UPDATE categories parent
        SET child_count = agg.count, updated_at = NOW()
        FROM (
            SELECT parent_id, COUNT(*)::INT AS count
            FROM categories
            WHERE parent_id IS NOT NULL
            GROUP BY parent_id
        ) agg
        WHERE parent.id = agg.parent_id
        "#
    )
    .execute(&mut *tx)
    .await
    .context("failed to sync category child_count")?;

    tx.commit().await.context("failed to commit catalog seed")?;
    Ok(())
}

async fn seed_wallets(pool: &PgPool) -> Result<()> {
    for wallet in WALLETS {
        let user_id = parse_uuid(wallet.user_id, "wallet user_id")?;
        sqlx::query(
            r#"
            INSERT INTO wallets (user_id, balance, held_balance, created_at, updated_at)
            VALUES ($1, $2, $3, NOW(), NOW())
            ON CONFLICT (user_id)
            DO UPDATE SET
                balance = EXCLUDED.balance,
                held_balance = EXCLUDED.held_balance,
                updated_at = NOW()
            "#,
        )
        .bind(user_id)
        .bind(wallet.balance)
        .bind(wallet.held_balance)
        .execute(pool)
        .await
        .with_context(|| format!("failed to upsert wallet for user_id={}", wallet.user_id))?;
    }

    Ok(())
}

async fn seed_auctions_and_bids(pool: &PgPool) -> Result<()> {
    let now = Utc::now();
    let mut tx = pool
        .begin()
        .await
        .context("failed to begin auction seed transaction")?;

    for auction in AUCTIONS {
        let auction_id = parse_uuid(auction.id, "auction id")?;
        let listing_id = parse_uuid(auction.listing_id, "auction listing_id")?;
        let seller_id = parse_uuid(auction.seller_id, "auction seller_id")?;
        let winner_id = match auction.winner_id {
            Some(value) => Some(parse_uuid(value, "auction winner_id")?),
            None => None,
        };

        let starts_at = now + Duration::hours(auction.starts_offset_hours);
        let ends_at = now + Duration::hours(auction.ends_offset_hours);
        let original_ends_at = now + Duration::hours(auction.original_ends_offset_hours);

        sqlx::query(
            r#"
            INSERT INTO auctions (
                id, listing_id, seller_id, seller_name, title, description, image_url,
                start_price, current_price, reserve_price, bid_increment, bid_count,
                status, winner_id, winner_name, starts_at, ends_at, original_ends_at,
                extension_count, created_at
            )
            VALUES (
                $1, $2, $3, $4, $5, $6, $7,
                $8, $9, $10, $11, $12,
                $13::auction_status, $14, $15, $16, $17, $18,
                $19, NOW()
            )
            ON CONFLICT (id)
            DO UPDATE SET
                listing_id = EXCLUDED.listing_id,
                seller_id = EXCLUDED.seller_id,
                seller_name = EXCLUDED.seller_name,
                title = EXCLUDED.title,
                description = EXCLUDED.description,
                image_url = EXCLUDED.image_url,
                start_price = EXCLUDED.start_price,
                current_price = EXCLUDED.current_price,
                reserve_price = EXCLUDED.reserve_price,
                bid_increment = EXCLUDED.bid_increment,
                bid_count = EXCLUDED.bid_count,
                status = EXCLUDED.status,
                winner_id = EXCLUDED.winner_id,
                winner_name = EXCLUDED.winner_name,
                starts_at = EXCLUDED.starts_at,
                ends_at = EXCLUDED.ends_at,
                original_ends_at = EXCLUDED.original_ends_at,
                extension_count = EXCLUDED.extension_count
            "#,
        )
        .bind(auction_id)
        .bind(listing_id)
        .bind(seller_id)
        .bind(auction.seller_name)
        .bind(auction.title)
        .bind(auction.description)
        .bind(auction.image_url)
        .bind(auction.start_price)
        .bind(auction.current_price)
        .bind(auction.reserve_price)
        .bind(auction.bid_increment)
        .bind(auction.bid_count)
        .bind(auction.status)
        .bind(winner_id)
        .bind(auction.winner_name)
        .bind(starts_at)
        .bind(ends_at)
        .bind(original_ends_at)
        .bind(auction.extension_count)
        .execute(&mut *tx)
        .await
        .with_context(|| format!("failed to upsert auction id={}", auction.id))?;

        sqlx::query(
            r#"
            UPDATE listings
            SET
                auction_id = $2,
                status = $3::listing_status,
                current_price = $4,
                bid_count = $5,
                starts_at = $6,
                ends_at = $7,
                updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(listing_id)
        .bind(auction_id)
        .bind(auction.listing_status)
        .bind(auction.current_price)
        .bind(auction.bid_count)
        .bind(starts_at)
        .bind(ends_at)
        .execute(&mut *tx)
        .await
        .with_context(|| {
            format!(
                "failed to link auction for listing_id={}",
                auction.listing_id
            )
        })?;

        sqlx::query("DELETE FROM bids WHERE auction_id = $1")
            .bind(auction_id)
            .execute(&mut *tx)
            .await
            .with_context(|| format!("failed to clear bids for auction_id={}", auction.id))?;

        sqlx::query("DELETE FROM proxy_bids WHERE auction_id = $1")
            .bind(auction_id)
            .execute(&mut *tx)
            .await
            .with_context(|| format!("failed to clear proxy bids for auction_id={}", auction.id))?;
    }

    for bid in BIDS {
        let bid_id = parse_uuid(bid.id, "bid id")?;
        let auction_id = parse_uuid(bid.auction_id, "bid auction_id")?;
        let bidder_id = parse_uuid(bid.bidder_id, "bid bidder_id")?;
        let created_at = now + Duration::minutes(bid.created_offset_minutes);

        sqlx::query(
            r#"
            INSERT INTO bids (
                id, auction_id, bidder_id, bidder_name, amount, is_proxy, is_winning, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            ON CONFLICT (id)
            DO UPDATE SET
                auction_id = EXCLUDED.auction_id,
                bidder_id = EXCLUDED.bidder_id,
                bidder_name = EXCLUDED.bidder_name,
                amount = EXCLUDED.amount,
                is_proxy = EXCLUDED.is_proxy,
                is_winning = EXCLUDED.is_winning,
                created_at = EXCLUDED.created_at
            "#,
        )
        .bind(bid_id)
        .bind(auction_id)
        .bind(bidder_id)
        .bind(bid.bidder_name)
        .bind(bid.amount)
        .bind(bid.is_proxy)
        .bind(bid.is_winning)
        .bind(created_at)
        .execute(&mut *tx)
        .await
        .with_context(|| format!("failed to upsert bid id={}", bid.id))?;
    }

    for proxy_bid in PROXY_BIDS {
        let proxy_bid_id = parse_uuid(proxy_bid.id, "proxy bid id")?;
        let auction_id = parse_uuid(proxy_bid.auction_id, "proxy bid auction_id")?;
        let bidder_id = parse_uuid(proxy_bid.bidder_id, "proxy bid bidder_id")?;
        let created_at = now + Duration::minutes(proxy_bid.created_offset_minutes);

        sqlx::query(
            r#"
            INSERT INTO proxy_bids (id, auction_id, bidder_id, max_amount, is_active, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, NOW())
            ON CONFLICT (id)
            DO UPDATE SET
                auction_id = EXCLUDED.auction_id,
                bidder_id = EXCLUDED.bidder_id,
                max_amount = EXCLUDED.max_amount,
                is_active = EXCLUDED.is_active,
                created_at = EXCLUDED.created_at,
                updated_at = NOW()
            "#,
        )
        .bind(proxy_bid_id)
        .bind(auction_id)
        .bind(bidder_id)
        .bind(proxy_bid.max_amount)
        .bind(proxy_bid.is_active)
        .bind(created_at)
        .execute(&mut *tx)
        .await
        .with_context(|| format!("failed to upsert proxy bid id={}", proxy_bid.id))?;
    }

    tx.commit()
        .await
        .context("failed to commit auctions and bids seed")?;
    Ok(())
}

async fn seed_orders_and_disputes(pool: &PgPool) -> Result<()> {
    let now = Utc::now();
    let mut tx = pool
        .begin()
        .await
        .context("failed to begin orders/disputes seed transaction")?;

    for order in ORDERS {
        let order_id = parse_uuid(order.id, "order id")?;
        let auction_id = parse_uuid(order.auction_id, "order auction_id")?;
        let listing_id = parse_uuid(order.listing_id, "order listing_id")?;
        let buyer_id = parse_uuid(order.buyer_id, "order buyer_id")?;
        let seller_id = parse_uuid(order.seller_id, "order seller_id")?;

        let created_at = now + Duration::hours(order.created_offset_hours);
        let updated_at = now + Duration::hours(order.updated_offset_hours);
        let estimated_delivery_at = order
            .estimated_delivery_offset_hours
            .map(|offset| now + Duration::hours(offset));
        let paid_at = order
            .paid_offset_hours
            .map(|offset| now + Duration::hours(offset));
        let shipped_at = order
            .shipped_offset_hours
            .map(|offset| now + Duration::hours(offset));
        let delivered_at = order
            .delivered_offset_hours
            .map(|offset| now + Duration::hours(offset));
        let confirmed_at = order
            .confirmed_offset_hours
            .map(|offset| now + Duration::hours(offset));

        sqlx::query("DELETE FROM orders WHERE auction_id = $1 AND id <> $2")
            .bind(auction_id)
            .bind(order_id)
            .execute(&mut *tx)
            .await
            .with_context(|| {
                format!(
                    "failed to normalize existing order rows for auction_id={}",
                    order.auction_id
                )
            })?;

        sqlx::query(
            r#"
            INSERT INTO orders (
                id, auction_id, listing_id, buyer_id, buyer_name, seller_id, seller_name,
                title, image_url, final_price, status, shipping_status,
                carrier, tracking_number, estimated_delivery_at,
                paid_at, shipped_at, delivered_at, confirmed_at,
                is_disputed, created_at, updated_at
            )
            VALUES (
                $1, $2, $3, $4, $5, $6, $7,
                $8, $9, $10, $11::order_status, $12::shipping_status,
                $13, $14, $15,
                $16, $17, $18, $19,
                $20, $21, $22
            )
            ON CONFLICT (id)
            DO UPDATE SET
                auction_id = EXCLUDED.auction_id,
                listing_id = EXCLUDED.listing_id,
                buyer_id = EXCLUDED.buyer_id,
                buyer_name = EXCLUDED.buyer_name,
                seller_id = EXCLUDED.seller_id,
                seller_name = EXCLUDED.seller_name,
                title = EXCLUDED.title,
                image_url = EXCLUDED.image_url,
                final_price = EXCLUDED.final_price,
                status = EXCLUDED.status,
                shipping_status = EXCLUDED.shipping_status,
                carrier = EXCLUDED.carrier,
                tracking_number = EXCLUDED.tracking_number,
                estimated_delivery_at = EXCLUDED.estimated_delivery_at,
                paid_at = EXCLUDED.paid_at,
                shipped_at = EXCLUDED.shipped_at,
                delivered_at = EXCLUDED.delivered_at,
                confirmed_at = EXCLUDED.confirmed_at,
                is_disputed = EXCLUDED.is_disputed,
                created_at = EXCLUDED.created_at,
                updated_at = EXCLUDED.updated_at
            "#,
        )
        .bind(order_id)
        .bind(auction_id)
        .bind(listing_id)
        .bind(buyer_id)
        .bind(order.buyer_name)
        .bind(seller_id)
        .bind(order.seller_name)
        .bind(order.title)
        .bind(order.image_url)
        .bind(order.final_price)
        .bind(order.status)
        .bind(order.shipping_status)
        .bind(order.carrier)
        .bind(order.tracking_number)
        .bind(estimated_delivery_at)
        .bind(paid_at)
        .bind(shipped_at)
        .bind(delivered_at)
        .bind(confirmed_at)
        .bind(order.is_disputed)
        .bind(created_at)
        .bind(updated_at)
        .execute(&mut *tx)
        .await
        .with_context(|| format!("failed to upsert order id={}", order.id))?;
    }

    for dispute in DISPUTES {
        let dispute_id = parse_uuid(dispute.id, "dispute id")?;
        let order_id = parse_uuid(dispute.order_id, "dispute order_id")?;
        let opened_by = parse_uuid(dispute.opened_by, "dispute opened_by")?;
        let created_at = now + Duration::hours(dispute.created_offset_hours);
        let resolved_at = dispute
            .resolved_offset_hours
            .map(|offset| now + Duration::hours(offset));

        sqlx::query("DELETE FROM disputes WHERE order_id = $1 AND id <> $2")
            .bind(order_id)
            .bind(dispute_id)
            .execute(&mut *tx)
            .await
            .with_context(|| {
                format!(
                    "failed to normalize existing dispute rows for order_id={}",
                    dispute.order_id
                )
            })?;

        sqlx::query(
            r#"
            INSERT INTO disputes (
                id, order_id, opened_by, reason, description, status, resolution, created_at, resolved_at
            )
            VALUES (
                $1, $2, $3, $4::dispute_reason, $5, $6::dispute_status, $7, $8, $9
            )
            ON CONFLICT (id)
            DO UPDATE SET
                order_id = EXCLUDED.order_id,
                opened_by = EXCLUDED.opened_by,
                reason = EXCLUDED.reason,
                description = EXCLUDED.description,
                status = EXCLUDED.status,
                resolution = EXCLUDED.resolution,
                created_at = EXCLUDED.created_at,
                resolved_at = EXCLUDED.resolved_at
            "#,
        )
        .bind(dispute_id)
        .bind(order_id)
        .bind(opened_by)
        .bind(dispute.reason)
        .bind(dispute.description)
        .bind(dispute.status)
        .bind(dispute.resolution)
        .bind(created_at)
        .bind(resolved_at)
        .execute(&mut *tx)
        .await
        .with_context(|| format!("failed to upsert dispute id={}", dispute.id))?;
    }

    tx.commit()
        .await
        .context("failed to commit orders and disputes seed")?;
    Ok(())
}

async fn seed_wallet_transactions(pool: &PgPool) -> Result<()> {
    let now = Utc::now();

    for transaction in WALLET_TRANSACTIONS {
        let tx_id = parse_uuid(transaction.id, "wallet transaction id")?;
        let wallet_id = parse_uuid(transaction.wallet_id, "wallet transaction wallet_id")?;
        let reference_id = match transaction.reference_id {
            Some(value) => Some(parse_uuid(value, "wallet transaction reference_id")?),
            None => None,
        };
        let created_at = now + Duration::hours(transaction.created_offset_hours);
        let completed_at = transaction
            .completed_offset_hours
            .map(|offset| now + Duration::hours(offset));

        sqlx::query(
            r#"
            INSERT INTO wallet_transactions (
                id, wallet_id, type, status, amount, balance_after,
                reference_id, reference_type, description, created_at, completed_at
            )
            VALUES (
                $1, $2, $3::transaction_type, $4::transaction_status, $5, $6,
                $7, $8::reference_type, $9, $10, $11
            )
            ON CONFLICT (id)
            DO UPDATE SET
                wallet_id = EXCLUDED.wallet_id,
                type = EXCLUDED.type,
                status = EXCLUDED.status,
                amount = EXCLUDED.amount,
                balance_after = EXCLUDED.balance_after,
                reference_id = EXCLUDED.reference_id,
                reference_type = EXCLUDED.reference_type,
                description = EXCLUDED.description,
                created_at = EXCLUDED.created_at,
                completed_at = EXCLUDED.completed_at
            "#,
        )
        .bind(tx_id)
        .bind(wallet_id)
        .bind(transaction.tx_type)
        .bind(transaction.status)
        .bind(transaction.amount)
        .bind(transaction.balance_after)
        .bind(reference_id)
        .bind(transaction.reference_type)
        .bind(transaction.description)
        .bind(created_at)
        .bind(completed_at)
        .execute(pool)
        .await
        .with_context(|| format!("failed to upsert wallet transaction id={}", transaction.id))?;
    }

    Ok(())
}

async fn seed_notifications(pool: &PgPool) -> Result<()> {
    let now = Utc::now();

    for notification in NOTIFICATIONS {
        let notification_id = parse_uuid(notification.id, "notification id")?;
        let user_id = parse_uuid(notification.user_id, "notification user_id")?;
        let reference_id = match notification.reference_id {
            Some(value) => Some(parse_uuid(value, "notification reference_id")?),
            None => None,
        };
        let created_at = now + Duration::hours(notification.created_offset_hours);
        let read_at = if notification.is_read {
            notification
                .read_offset_hours
                .map(|offset| now + Duration::hours(offset))
                .or(Some(now))
        } else {
            None
        };

        sqlx::query(
            r#"
            INSERT INTO notifications (
                id, user_id, type, title, message, is_read, reference_id, reference_type, read_at, created_at
            )
            VALUES (
                $1, $2, $3::notification_type, $4, $5, $6, $7, $8::reference_type, $9, $10
            )
            ON CONFLICT (id)
            DO UPDATE SET
                user_id = EXCLUDED.user_id,
                type = EXCLUDED.type,
                title = EXCLUDED.title,
                message = EXCLUDED.message,
                is_read = EXCLUDED.is_read,
                reference_id = EXCLUDED.reference_id,
                reference_type = EXCLUDED.reference_type,
                read_at = EXCLUDED.read_at,
                created_at = EXCLUDED.created_at
            "#,
        )
        .bind(notification_id)
        .bind(user_id)
        .bind(notification.notification_type)
        .bind(notification.title)
        .bind(notification.message)
        .bind(notification.is_read)
        .bind(reference_id)
        .bind(notification.reference_type)
        .bind(read_at)
        .bind(created_at)
        .execute(pool)
        .await
        .with_context(|| format!("failed to upsert notification id={}", notification.id))?;
    }

    Ok(())
}

fn parse_uuid(raw: &str, field: &str) -> Result<Uuid> {
    Uuid::parse_str(raw).with_context(|| format!("invalid {field}: {raw}"))
}
