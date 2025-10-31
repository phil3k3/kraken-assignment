extern crate core;
mod account;
mod error;
mod prelude;
mod reader;

use crate::reader::{process_csv, write_accounts};
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    let program = args.get(0).unwrap();
    if args.len() != 2 {
        panic!("Usage: {program} <csv file>");
    }
    process_csv(args.get(1).unwrap())
        .and_then(|x| {
            write_accounts(x).and_then(|x| {
                print!("{}", x);
                Ok(())
            })
        })
        .unwrap();
}
