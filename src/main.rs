extern crate core;
mod account;
mod error;
mod prelude;
mod reader;
mod settings;

use crate::reader::{parse_csv, write_accounts};
use crate::settings::Settings;
use std::env;
use primitive_fixed_point_decimal::ConstScaleFpdec;

type Amount = ConstScaleFpdec<i64, 4>;

fn main() {
    let args: Vec<String> = env::args().collect();
    let program = args.first().expect("program name not available");
    if args.len() != 2 {
        eprintln!("Usage: {program} <csv file>");
        std::process::exit(1);
    }

    let settings = Settings::new().unwrap_or_else(|err| {
        eprintln!("Warning: Failed to load settings: {err}. Using defaults.");
        Settings::default()
    });

    parse_csv(args.get(1).expect("csv file argument"), settings.buffer_capacity())
        .and_then(|accounts| {
            write_accounts(accounts).map(|output| {
                print!("{}", output);
            })
        })
        .unwrap_or_else(|err| {
            eprintln!("Error: {err}");
            std::process::exit(1);
        });
}
