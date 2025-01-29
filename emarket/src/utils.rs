use chrono::{DateTime, Datelike, Days, NaiveDateTime, TimeZone};
use chrono_tz::Europe::Vilnius;
use rand::Rng;

pub fn to_time(t: u64) -> NaiveDateTime {
    DateTime::from_timestamp_millis(i64::try_from(t).unwrap())
        .expect("wrong millis")
        .naive_utc()
}

pub fn time_day_vilnius(time: NaiveDateTime, shift_days: i64) -> NaiveDateTime {
    let nt = if shift_days >= 0 {
        time.checked_add_days(Days::new(shift_days as u64)).unwrap()
    } else {
        time.checked_sub_days(Days::new((-shift_days) as u64))
            .unwrap()
    };
    let nt = Vilnius.from_utc_datetime(&nt);
    let dt = Vilnius
        .with_ymd_and_hms(nt.year(), nt.month(), nt.day(), 0, 0, 0)
        .latest()
        .unwrap();
    dt.naive_utc()
}

pub fn time_month_vilnius(time: NaiveDateTime, shift_months: i32) -> NaiveDateTime {
    let dtz = Vilnius.from_utc_datetime(&time);
    let m = (dtz.month() as i32 - 1) + shift_months;
    let (ys, nm) = if m < 0 {
        (-1 + m / 12, (12 + m % 12) as u32)
    } else {
        (m / 12, (m % 12) as u32)
    };
    let dt = Vilnius
        .with_ymd_and_hms(dtz.year() + ys, nm + 1, 1, 0, 0, 0)
        .latest()
        .unwrap();
    dt.naive_utc()
}

pub fn jitter(d: chrono::Duration) -> chrono::Duration {
    let mut rng = rand::thread_rng();
    let ms = rng.gen_range(0..d.num_milliseconds());
    chrono::Duration::milliseconds(ms)
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, NaiveDate, NaiveDateTime};
    use more_asserts::{assert_ge, assert_le};

    use crate::utils::{jitter, time_day_vilnius, time_month_vilnius, to_time};
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

    macro_rules! day_tests {
        ($($name:ident: $value:expr,)*) => {
        $(
            #[test]
            fn $name() {
                let (input, shift, expected) = $value;
                assert_eq!(expected, time_day_vilnius(input, shift));
            }
        )*
        }
    }

    day_tests! {
        d_today: (dt(2023, 1, 1, 21, 0, 0), 0, dt(2022, 12, 31, 22, 0, 0)),
        d_today_1: (dt(2023, 1, 1, 23, 0, 0), 0, dt(2023, 1, 1, 22, 0, 0)),
        d_next: (dt(2023, 1, 1, 21, 0, 0), 1, dt(2023, 1, 1, 22, 0, 0)),
        d_next_1: (dt(2023, 1, 2, 10, 0, 1), 1, dt(2023, 1, 2, 22, 0, 0)),
        d_next_2: (dt(2023, 4, 2, 10, 0, 0), 1, dt(2023, 4, 2, 21, 0, 0)),
        d_prev: (dt(2023, 1, 1, 21, 0, 0), -1, dt(2022, 12, 30, 22, 0, 0)),
        d_prev_1: (dt(2023, 1, 3, 10, 0, 1), -1, dt(2023, 1, 1, 22, 0, 0)),
        d_prev_2: (dt(2023, 4, 3, 10, 0, 1), -1, dt(2023, 4, 1, 21, 0, 0)),
        d_prev_3: (dt(2023, 1, 1, 22, 0, 0), -7, dt(2022, 12, 25, 22, 0, 0)),
        d_prev_4: (dt(2023, 2, 3, 10, 0, 1), -30, dt(2023, 1, 3, 22, 0, 0)),
        d_prev_5: (dt(2023, 4, 3, 10, 0, 1), -30, dt(2023, 3, 3, 22, 0, 0)),
    }

    macro_rules! month_tests {
        ($($name:ident: $value:expr,)*) => {
        $(
            #[test]
            fn $name() {
                let (input, shift, expected) = $value;
                assert_eq!(expected, time_month_vilnius(input, shift));
            }
        )*
        }
    }

    month_tests! {
        m_today: (dt(2023, 1, 1, 21, 0, 0), 0, dt(2022, 12, 31, 22, 0, 0)),
        m_today_1: (dt(2023, 4, 1, 21, 0, 0), 0, dt(2023, 3, 31, 21, 0, 0)),
        m_next: (dt(2023, 1, 1, 22, 0, 0), 1, dt(2023, 1, 31, 22, 0, 0)),
        m_next_2: (dt(2023, 1, 5, 22, 0, 0), 1, dt(2023, 1, 31, 22, 0, 0)),
        m_next_3: (dt(2023, 1, 31, 21, 0, 0), 1, dt(2023, 1, 31, 22, 0, 0)),
        m_next_4: (dt(2023, 1, 31, 23, 0, 0), 1, dt(2023, 2, 28, 22, 0, 0)),
        m_next_5: (dt(2023, 2, 1, 0, 0, 0), 1, dt(2023, 2, 28, 22, 0, 0)),
        m_next_6: (dt(2023, 3, 1, 0, 0, 0), 1, dt(2023, 3, 31, 21, 0, 0)),
        m_prev: (dt(2023, 1, 1, 21, 0, 0), -1, dt(2022, 11, 30, 22, 0, 0)),
        m_prev_1: (dt(2023, 4, 1, 22, 0, 0), -1, dt(2023, 2, 28, 22, 0, 0)),
    }

    fn dt(year: i32, month: u32, day: u32, h: u32, m: u32, s: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(year, month, day)
            .unwrap()
            .and_hms_opt(h, m, s)
            .unwrap()
    }
}
