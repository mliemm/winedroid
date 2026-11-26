use super::abi_types::{EOVERFLOW, TM_GMTOFF_OFFSET, TM_SIZE, TM_ZONE_OFFSET};
use super::guest_mem::{read_value, write_bytes, write_value};

const SECONDS_PER_DAY: i64 = 86_400;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BrokenDown {
    pub second: i32,
    pub minute: i32,
    pub hour: i32,
    pub day: i32,
    pub month: i32,
    pub year: i32,
    pub weekday: i32,
    pub yearday: i32,
}

pub fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let shifted = if month <= 2 { year - 1 } else { year };
    let era = shifted.div_euclid(400);
    let year_of_era = shifted - era * 400;
    let month_index = (month + 9) % 12;
    let day_of_year = (153 * month_index + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let shifted = days + 719_468;
    let era = shifted.div_euclid(146_097);
    let day_of_era = shifted - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_index = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_index + 2) / 5 + 1;
    let month = if month_index < 10 {
        month_index + 3
    } else {
        month_index - 9
    };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

pub fn from_seconds(seconds: i64) -> Result<BrokenDown, i32> {
    // cite: bionic localtime.c#timesub
    let days = seconds.div_euclid(SECONDS_PER_DAY);
    let remainder = seconds.rem_euclid(SECONDS_PER_DAY);
    let (year, month, day) = civil_from_days(days);
    let tm_year = i32::try_from(year - 1900).map_err(|_| EOVERFLOW)?;
    Ok(BrokenDown {
        second: (remainder % 60) as i32,
        minute: (remainder / 60 % 60) as i32,
        hour: (remainder / 3600) as i32,
        day: day as i32,
        month: month as i32 - 1,
        year: tm_year,
        weekday: (days + 4).rem_euclid(7) as i32,
        yearday: (days - days_from_civil(year, 1, 1)) as i32,
    })
}

pub fn to_seconds(parts: &BrokenDown) -> Result<i64, i32> {
    // cite: bionic localtime.c#time1, time2sub
    let month_total = i128::from(parts.month);
    let year = i128::from(parts.year) + 1900 + month_total.div_euclid(12);
    let month = month_total.rem_euclid(12) + 1;
    let days = i128::from(days_from_civil(
        i64::try_from(year).map_err(|_| EOVERFLOW)?,
        month as i64,
        1,
    )) + i128::from(parts.day)
        - 1;
    let total = days * 86_400
        + i128::from(parts.hour) * 3600
        + i128::from(parts.minute) * 60
        + i128::from(parts.second);
    i64::try_from(total).map_err(|_| EOVERFLOW)
}

pub fn read_tm(address: usize) -> BrokenDown {
    let field = |index: usize| read_value::<i32>(address + index * 4);
    BrokenDown {
        second: field(0),
        minute: field(1),
        hour: field(2),
        day: field(3),
        month: field(4),
        year: field(5),
        weekday: field(6),
        yearday: field(7),
    }
}

pub fn write_tm(address: usize, parts: &BrokenDown, is_dst: i32, offset: i64, zone_name: usize) {
    let mut block = [0u8; TM_SIZE];
    let fields = [
        parts.second,
        parts.minute,
        parts.hour,
        parts.day,
        parts.month,
        parts.year,
        parts.weekday,
        parts.yearday,
        is_dst,
    ];
    for (index, value) in fields.iter().enumerate() {
        block[index * 4..index * 4 + 4].copy_from_slice(&value.to_le_bytes());
    }
    write_bytes(address, &block);
    write_value::<i64>(address + TM_GMTOFF_OFFSET, offset);
    write_value::<usize>(address + TM_ZONE_OFFSET, zone_name);
}
