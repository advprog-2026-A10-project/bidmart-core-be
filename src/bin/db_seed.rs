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

#[tokio::main]
async fn main() -> Result<()> {
    init_tracer();
    dotenv::dotenv().ok();

    let config = AppConfig::new().context("failed to load app config")?;
    let pool = create_pool(&config.database_url).await?;
    run_pending_migrations(&pool).await?;

    seed_catalog(&pool).await?;

    tracing::info!(
        categories = CATEGORIES.len(),
        listings = LISTINGS.len(),
        "Catalog seed completed"
    );
    println!(
        "Catalog seed completed: {} categories and {} listings upserted.",
        CATEGORIES.len(),
        LISTINGS.len()
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

fn parse_uuid(raw: &str, field: &str) -> Result<Uuid> {
    Uuid::parse_str(raw).with_context(|| format!("invalid {field}: {raw}"))
}
