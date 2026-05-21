use super::{ProxyCapability, compute_minimum_proxy_amount, sort_proxy_capabilities};
use chrono::{TimeZone, Utc};
use uuid::Uuid;

#[test]
fn sort_proxy_capabilities_uses_max_then_priority_then_uuid() {
    let t0 = Utc.with_ymd_and_hms(2026, 5, 19, 0, 0, 0).unwrap();
    let t1 = Utc.with_ymd_and_hms(2026, 5, 19, 0, 1, 0).unwrap();

    let bidder_a = Uuid::from_u128(1);
    let bidder_b = Uuid::from_u128(2);
    let bidder_c = Uuid::from_u128(3);

    let mut capabilities = vec![
        ProxyCapability {
            bidder_id: bidder_b,
            bidder_name: "B".to_string(),
            max_amount: 150,
            priority_at: t1,
            has_active_proxy: true,
        },
        ProxyCapability {
            bidder_id: bidder_a,
            bidder_name: "A".to_string(),
            max_amount: 150,
            priority_at: t1,
            has_active_proxy: true,
        },
        ProxyCapability {
            bidder_id: bidder_c,
            bidder_name: "C".to_string(),
            max_amount: 200,
            priority_at: t0,
            has_active_proxy: true,
        },
    ];

    sort_proxy_capabilities(&mut capabilities);
    let ordered_ids: Vec<Uuid> = capabilities.iter().map(|item| item.bidder_id).collect();

    assert_eq!(ordered_ids, vec![bidder_c, bidder_a, bidder_b]);
}

#[test]
fn compute_minimum_proxy_amount_matches_leading_rule() {
    assert_eq!(compute_minimum_proxy_amount(1_000, 100, false, None), 1_100);
    assert_eq!(
        compute_minimum_proxy_amount(1_000, 100, true, Some(1_250)),
        1_250
    );
    assert_eq!(compute_minimum_proxy_amount(1_000, 100, true, None), 1_000);
}
