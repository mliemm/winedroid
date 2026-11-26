use super::export::in_libc;
use super::locale::current_max_bytes;

const CODESET: usize = 1;
const NAMED_ITEMS: usize = 56;

const DAYS: [&str; 7] = [
    "Sunday",
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
];
const DAYS_SHORT: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];
const MONTHS_SHORT: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

fn item_text(item: usize) -> String {
    // cite: bionic langinfo.cpp#nl_langinfo
    let first = |names: &[&str], first_item: usize| names[item - first_item].to_string();
    match item {
        CODESET => if current_max_bytes() == 1 {
            "ASCII"
        } else {
            "UTF-8"
        }
        .to_string(),
        2 => "%F %T %z".to_string(),
        3 => "%F".to_string(),
        4 => "%T".to_string(),
        5 => "%I:%M:%S %p".to_string(),
        6 => "AM".to_string(),
        7 => "PM".to_string(),
        8..=14 => first(&DAYS, 8),
        15..=21 => first(&DAYS_SHORT, 15),
        22..=33 => first(&MONTHS, 22),
        34..=45 => first(&MONTHS_SHORT, 34),
        51 => ".".to_string(),
        53 => "^[yY]".to_string(),
        54 => "^[nN]".to_string(),
        _ => String::new(),
    }
}

fn answer(item: i32) -> usize {
    let index = usize::try_from(item)
        .ok()
        .filter(|index| *index < NAMED_ITEMS);
    super::guest_mem::intern(&index.map_or_else(String::new, item_text))
}

pub fn register() {
    in_libc("nl_langinfo", |context| {
        context.ret_ptr(answer(context.arg_i32(0)))
    });
    in_libc("nl_langinfo_l", |context| {
        context.ret_ptr(answer(context.arg_i32(0)))
    });
}
