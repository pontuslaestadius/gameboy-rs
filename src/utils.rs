use crate::Instruction;
use human_format::{Formatter, Scales};
use log::info;
use std::fs::OpenOptions;
use std::io;
use std::io::prelude::*;

pub fn pretty<T: Into<f64>>(size: T) -> String {
    Formatter::new().with_decimals(2).format(size.into())
}

pub fn print_header(str: String) {
    let mut padding = String::new();
    for _ in 0..(24 - str.len() / 2) {
        padding.push('-');
    }
    info!("{1} {0} {1}", str, padding);
}

/// Pretty-formatting size.
pub fn print_size(size: usize) -> String {
    let mut scales = Scales::new();
    scales.with_base(1024).with_suffixes(vec!["B", "kB", "MB"]);
    Formatter::new().with_scales(scales).format(size as f64)
}

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

/// Writes the given vec to the given path.
pub fn write_vec(path: &str, vec: &Vec<Instruction>) -> Result<(), io::Error> {
    let mut file = OpenOptions::new().write(true).create(true).open(path)?;

    for item in vec.iter() {
        file.write_all(format!("{:?}", item).as_bytes())?;
        file.write(b"\n")?;
    }
    Ok(())
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
