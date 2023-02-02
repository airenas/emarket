use chrono::NaiveDateTime;
use rand::Rng;

pub fn to_time(t: u64) -> NaiveDateTime {
    NaiveDateTime::from_timestamp_millis(i64::try_from(t).unwrap()).expect("wrong millis")
}

pub fn jitter(d: chrono::Duration) -> chrono::Duration {
    let mut rng = rand::thread_rng();
    let ms = rng.gen_range(0..d.num_milliseconds());
    chrono::Duration::milliseconds(ms)
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, NaiveDate};
    use more_asserts::{assert_ge, assert_le};

    use crate::utils::{jitter, to_time};
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
    #[test]
    fn jitter_test() {
        let mut gt_than_middle = 0;
        for _ in 0..100 {
            let res = jitter(Duration::minutes(10));
            assert_le!(res, Duration::minutes(10));
            assert_ge!(res, Duration::minutes(0));
            if res > Duration::minutes(5) {
                gt_than_middle += 1;
            }
        }
        assert_le!(gt_than_middle, 70);
        assert_ge!(gt_than_middle, 30);
    }
}
