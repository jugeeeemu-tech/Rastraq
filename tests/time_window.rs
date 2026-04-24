use chrono::{TimeZone, Utc};
use rastraq::time::previous_local_date;

#[test]
fn previous_local_date_uses_user_timezone() {
    let now_utc = Utc.with_ymd_and_hms(2026, 4, 24, 14, 30, 0).unwrap();

    let target = previous_local_date(now_utc, "Asia/Tokyo").unwrap();

    assert_eq!(target.to_string(), "2026-04-23");
}
