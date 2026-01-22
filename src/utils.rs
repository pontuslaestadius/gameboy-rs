pub fn to_hex<T: Into<u16>>(val: T) -> String {
    format!("{:01$x}", val.into(), 2)
}

pub fn int(val: &str) -> u16 {
    u16::from_str_radix(val, 16).unwrap()
}

pub fn str_to_code(code: &str) -> [Option<char>; 2] {
    let mut chars = code.chars();
    let first = chars.next();
    let second = chars.nth(1);
    [first, second]
}

pub fn octal_digit_from_binary_list(list: &[u8]) -> u8 {
    let mut multiplier = 1;
    let mut result: u8 = 0;

    for item in list.iter().rev() {
        result += item * multiplier;
        multiplier *= 2;
    }
    result
}

pub fn octal_digit_from_binary_list_u16(list: &[u8]) -> u16 {
    let mut multiplier: u32 = 1;
    let mut result: u16 = 0;

    for item in list.iter().rev() {
        result += *item as u16 * multiplier as u16;
        multiplier *= 2;
    }
    result
}

pub fn octal_digit_from_binary_list_i16(list: &[u8]) -> i16 {
    let mut result: i16 = 0;

    let mut iter = list.iter();
    let signed = iter.next().unwrap();

    let signed_clear: i16 = match *signed {
        0 => 1,
        _ => -1,
    };

    let two: i16 = 2;
    for (index, item) in iter.rev().enumerate() {
        result += *item as i16 * two.pow(index as u32);
    }
    result * signed_clear
}

pub fn output_string_diff(string_a: &str, string_b: &str) -> String {
    if string_a.len() != string_b.len() {
        panic!(
            "String_diff requires equal lengths. A: {}, B: {}",
            string_a.len(),
            string_b.len()
        );
    }

    // Zip pairs up characters: (a[0], b[0]), (a[1], b[1]), etc.
    string_a
        .chars()
        .zip(string_b.chars())
        .map(|(a, b)| if a == b { ' ' } else { b })
        .collect()
}
