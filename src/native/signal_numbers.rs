pub const SIGKILL: i32 = 9;
pub const SIGSTOP: i32 = 19;
pub const SIGNAL_COUNT: i32 = 64;
pub const RT_MIN: i32 = 32;
pub const RT_RESERVED: i32 = 9;

const MAPPED: [(i32, i32); 29] = [
    (1, 1),
    (2, 2),
    (3, 3),
    (4, 4),
    (5, 5),
    (6, 6),
    (7, 10),
    (8, 8),
    (9, 9),
    (10, 30),
    (11, 11),
    (12, 31),
    (13, 13),
    (14, 14),
    (15, 15),
    (17, 20),
    (18, 19),
    (19, 17),
    (20, 18),
    (21, 21),
    (22, 22),
    (23, 16),
    (24, 24),
    (25, 25),
    (26, 26),
    (27, 27),
    (28, 28),
    (29, 23),
    (31, 12),
];

pub fn linux_to_darwin(signal: i32) -> Option<i32> {
    // cite: bionic signal.h#SIGUSR1, SIGBUS, SIGCHLD, SIGSTOP, SIGTSTP, SIGCONT, SIGURG, SIGIO, SIGSYS
    MAPPED.iter().find(|(linux, _)| *linux == signal).map(|(_, darwin)| *darwin)
}

pub fn darwin_to_linux(signal: i32) -> Option<i32> {
    MAPPED.iter().find(|(_, darwin)| *darwin == signal).map(|(linux, _)| *linux)
}

pub fn is_valid(signal: i32) -> bool {
    (1..=SIGNAL_COUNT).contains(&signal)
}

pub fn is_virtual(signal: i32) -> bool {
    is_valid(signal) && linux_to_darwin(signal).is_none()
}

pub fn is_synchronous(signal: i32) -> bool {
    matches!(signal, 4 | 5 | 7 | 8 | 11 | 31)
}

pub fn bit(signal: i32) -> u64 {
    1u64 << (signal - 1)
}

pub fn to_darwin_mask(linux: u64) -> u32 {
    MAPPED.iter().filter(|(signal, _)| linux & bit(*signal) != 0).fold(0, |mask, (_, darwin)| mask | 1u32 << (darwin - 1))
}

pub fn to_linux_mask(darwin: u32) -> u64 {
    MAPPED.iter().filter(|(_, signal)| darwin & (1u32 << (signal - 1)) != 0).fold(0, |mask, (linux, _)| mask | bit(*linux))
}

pub fn filter_reserved(set: u64, how: i32) -> u64 {
    // cite: bionic reserved_signals.h#filter_reserved_signals
    let reserved = (RT_MIN + 1..RT_MIN + RT_RESERVED).fold(0, |mask, signal| mask | bit(signal));
    let timers = bit(RT_MIN);
    match how {
        1 => (set & !timers) | reserved,
        _ => (set | timers) & !reserved,
    }
}

pub fn rt_min() -> i32 {
    // cite: bionic __libc_current_sigrtmin.cpp#__libc_current_sigrtmin
    RT_MIN + RT_RESERVED
}

pub fn rt_max() -> i32 {
    // cite: bionic __libc_current_sigrtmax.cpp#__libc_current_sigrtmax
    SIGNAL_COUNT
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bionic_signal_numbers_map_to_distinct_darwin_numbers_and_round_trip() {
        // cite: bionic signal.h#SIGUSR1, SIGBUS, SIGCHLD
        assert_eq!(linux_to_darwin(7), Some(libc::SIGBUS));
        assert_eq!(linux_to_darwin(10), Some(libc::SIGUSR1));
        assert_eq!(linux_to_darwin(12), Some(libc::SIGUSR2));
        assert_eq!(linux_to_darwin(17), Some(libc::SIGCHLD));
        assert_eq!(linux_to_darwin(19), Some(libc::SIGSTOP));
        assert_eq!(linux_to_darwin(31), Some(libc::SIGSYS));
        for signal in 1..=SIGNAL_COUNT {
            if let Some(darwin) = linux_to_darwin(signal) {
                assert_eq!(darwin_to_linux(darwin), Some(signal));
            }
        }
        let darwin: Vec<_> = (1..=SIGNAL_COUNT).filter_map(linux_to_darwin).collect();
        let mut unique = darwin.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(unique.len(), darwin.len());
    }

    #[test]
    fn bionic_stack_fault_power_and_realtime_signals_have_no_darwin_number() {
        // cite: bionic signal.h#SIGSTKFLT, SIGPWR, __SIGRTMIN
        for signal in [16, 30, 32, 40, 64] {
            assert!(is_virtual(signal), "{signal}");
        }
        assert!(!is_virtual(11) && !is_valid(0) && !is_valid(65));
    }

    #[test]
    fn masks_translate_between_linux_64_bit_sets_and_darwin_32_bit_sets() {
        let linux = bit(7) | bit(10) | bit(16) | bit(40);

        assert_eq!(to_darwin_mask(linux), (1 << (libc::SIGBUS - 1)) | (1 << (libc::SIGUSR1 - 1)));
        assert_eq!(to_linux_mask(to_darwin_mask(linux)), bit(7) | bit(10));
    }

    #[test]
    fn reserved_signal_filter_matches_bionic_for_block_and_unblock() {
        // cite: bionic reserved_signals.h#filter_reserved_signals
        let reserved: u64 = (33..=40).map(bit).sum();

        assert_eq!(filter_reserved(0, 0), bit(32));
        assert_eq!(filter_reserved(u64::MAX, 2), u64::MAX & !reserved);
        assert_eq!(filter_reserved(bit(32), 1), reserved);
        assert_eq!(filter_reserved(0, 1), reserved);
        assert_eq!((rt_min(), rt_max()), (41, 64));
    }
}
