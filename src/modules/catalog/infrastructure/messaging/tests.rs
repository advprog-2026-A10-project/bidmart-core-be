use uuid::Uuid;

use super::BidPlacedEvent;

#[test]
fn deserialize_valid_event() {
    let listing_id = Uuid::new_v4();
    let json = format!(r#"{{"listing_id":"{}","new_price":150000}}"#, listing_id);
    let event: BidPlacedEvent = serde_json::from_str(&json).unwrap();
    assert_eq!(event.listing_id, listing_id);
    assert_eq!(event.new_price, 150000);
}

#[test]
fn deserialize_missing_listing_id_fails() {
    assert!(serde_json::from_str::<BidPlacedEvent>(r#"{"new_price":150000}"#).is_err());
}

#[test]
fn deserialize_missing_new_price_fails() {
    let json = format!(r#"{{"listing_id":"{}"}}"#, Uuid::new_v4());
    assert!(serde_json::from_str::<BidPlacedEvent>(&json).is_err());
}

#[test]
fn deserialize_invalid_uuid_fails() {
    assert!(serde_json::from_str::<BidPlacedEvent>(r#"{"listing_id":"bukan-uuid","new_price":150000}"#).is_err());
}

#[test]
fn deserialize_wrong_price_type_fails() {
    let json = format!(r#"{{"listing_id":"{}","new_price":"bukan_angka"}}"#, Uuid::new_v4());
    assert!(serde_json::from_str::<BidPlacedEvent>(&json).is_err());
}
