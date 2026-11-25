use super::abi_types::EINVAL;
use super::clocks;

pub const CLOCK_REALTIME: i32 = 0;
pub const CLOCK_MONOTONIC: i32 = 1;
pub const ETIMEDOUT: i32 = 110;

const NANOS_PER_SECOND: i128 = 1_000_000_000;
const LONGEST_NS: i128 = 1 << 62;

#[derive(Debug, Clone, Copy)]
pub struct Deadline {
    pub clock: i32,
    pub seconds: i64,
    pub nanos: i64,
}

impl Deadline {
    pub fn remaining_ns(&self) -> i64 {
        let Ok(now) = clocks::now(self.clock) else { return 0 };
        let seconds = i128::from(self.seconds) - i128::from(now.tv_sec);
        let nanos = i128::from(self.nanos) - i128::from(now.tv_nsec);
        (seconds * NANOS_PER_SECOND + nanos).clamp(-LONGEST_NS, LONGEST_NS) as i64
    }

    pub fn after_ns(clock: i32, nanos: i64) -> Self {
        let now = clocks::now(clock).unwrap_or(libc::timespec { tv_sec: 0, tv_nsec: 0 });
        let total = i128::from(now.tv_nsec) + i128::from(nanos);
        Self { clock, seconds: now.tv_sec.saturating_add(total.div_euclid(NANOS_PER_SECOND) as i64), nanos: total.rem_euclid(NANOS_PER_SECOND) as i64 }
    }
}

fn read_timespec(address: usize) -> (i64, i64) {
    super::guest_mem::read_value::<(i64, i64)>(address)
}

pub fn check_timespec(address: usize, null_allowed: bool) -> i32 {
    // cite: bionic bionic_time_conversions.h#check_timespec
    if address == 0 {
        return if null_allowed { 0 } else { EINVAL };
    }
    let (seconds, nanos) = read_timespec(address);
    if !(0..NANOS_PER_SECOND as i64).contains(&nanos) {
        return EINVAL;
    }
    if seconds < 0 {
        return ETIMEDOUT;
    }
    0
}

pub fn deadline_at(address: usize, clock: i32) -> Option<Deadline> {
    (address != 0).then(|| {
        let (seconds, nanos) = read_timespec(address);
        Deadline { clock, seconds, nanos }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(seconds: i64, nanos: i64) -> usize {
        Box::leak(Box::new((seconds, nanos))) as *const (i64, i64) as usize
    }

    #[test]
    fn check_timespec_follows_the_bionic_decision_order() {
        // cite: bionic bionic_time_conversions.h#check_timespec
        assert_eq!(check_timespec(0, true), 0);
        assert_eq!(check_timespec(spec(1, 1_000_000_000), true), EINVAL);
        assert_eq!(check_timespec(spec(1, -1), true), EINVAL);
        assert_eq!(check_timespec(spec(-1, 0), true), ETIMEDOUT);
        assert_eq!(check_timespec(spec(-1, 5_000_000_000), true), EINVAL);
        assert_eq!(check_timespec(spec(0, 999_999_999), true), 0);
    }

    #[test]
    fn deadlines_count_down_on_the_guest_clock() {
        let deadline = Deadline::after_ns(CLOCK_MONOTONIC, 500_000_000);

        let remaining = deadline.remaining_ns();

        assert!(remaining > 400_000_000 && remaining <= 500_000_000, "{remaining}");
        assert!(Deadline::after_ns(CLOCK_REALTIME, -1_000_000_000).remaining_ns() < 0);
    }
}
