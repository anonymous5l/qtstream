use crate::coremedia::time::Time;
use std::time::SystemTime;

const NANO_SECOND_SCALE: u32 = 1000000000;

const KCM_TIME_FLAGS_VALID: u32 = 0x0;
const KCM_TIME_FLAGS_HAS_BEEN_ROUNDED: u32 = 0x1;
const KCM_TIME_FLAGS_POSITIVE_INFINITY: u32 = 0x2;
const KCM_TIME_FLAGS_NEGATIVE_INFINITY: u32 = 0x4;
const KCM_TIME_FLAGS_INDEFINITE: u32 = 0x8;
const KCM_TIME_FLAGS_IMPLIED_VALUE_FLAGS_MASK: u32 =
    KCM_TIME_FLAGS_POSITIVE_INFINITY | KCM_TIME_FLAGS_NEGATIVE_INFINITY | KCM_TIME_FLAGS_INDEFINITE;

const TIME_LENGTH_IN_BYTES: i32 = 24;

pub struct Clock {
    id: u64,
    time_scale: u32,
    factor: f64,
    t: SystemTime,
}

impl Clock {
    pub fn new_with_host_time(id: u64) -> Clock {
        Clock {
            id,
            time_scale: NANO_SECOND_SCALE,
            factor: 1f64,
            t: SystemTime::now(),
        }
    }

    pub fn new_with_host_time_and_scale(id: u64, ts: u32) -> Clock {
        Clock {
            id,
            time_scale: ts,
            factor: ts as f64 / NANO_SECOND_SCALE as f64,
            t: SystemTime::now(),
        }
    }

    pub fn calculate_skew(st1: &Time, et1: &Time, st2: &Time, et2: &Time) -> f64 {
        let diff_clock1 = et1.value() - st1.value();
        let diff_clock2 = et2.value() - et1.value();

        let diff_time = Time::new(diff_clock1, st1.scale(), 0, 0);
        let scaled_diff = diff_time.get_time_for_scale(st2);

        (st2.scale() as f64) * scaled_diff / (diff_clock2 as f64)
    }

    pub fn get_time(&self) -> Time {
        let since = SystemTime::now()
            .duration_since(self.t)
            .expect("get time duration since");

        Time::new(
            self.calc_value(since.as_nanos() as u64),
            self.time_scale,
            KCM_TIME_FLAGS_HAS_BEEN_ROUNDED,
            0,
        )
    }

    fn calc_value(&self, val: u64) -> u64 {
        if NANO_SECOND_SCALE == self.time_scale {
            return val;
        }
        (self.factor * val as f64) as u64
    }
}
