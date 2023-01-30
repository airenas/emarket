use chrono::NaiveDateTime;

pub fn to_time(t: u64) -> NaiveDateTime {
    NaiveDateTime::from_timestamp_millis(i64::try_from(t).unwrap()).expect("wrong millis")
}

#[cfg(test)]
mod tests {
    use chrono::{NaiveDate};

    use crate::utils::to_time;
    #[test]
    fn to_time_0() {
        assert_eq!(
            to_time(0),
            NaiveDate::from_ymd_opt(1970, 1, 1)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap()
        );
    }
}
