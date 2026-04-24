use anyhow::{anyhow, Result};
use chrono::{DateTime, Days, NaiveDate, Utc};
use chrono_tz::Tz;

pub fn previous_local_date(now_utc: DateTime<Utc>, timezone: &str) -> Result<NaiveDate> {
    let tz: Tz = timezone
        .parse()
        .map_err(|_| anyhow!("invalid timezone: {timezone}"))?;
    now_utc
        .with_timezone(&tz)
        .date_naive()
        .checked_sub_days(Days::new(1))
        .ok_or_else(|| anyhow!("could not calculate previous local date"))
}
