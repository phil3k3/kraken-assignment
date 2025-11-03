extern crate core;
mod account;
mod error;
mod prelude;
mod reader;

use crate::reader::{process_csv, write_accounts};
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
    process_csv(args.get(1).expect("csv file argument"))
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
