extern crate core;
mod prelude;
mod error;
mod account;
mod reader;

use crate::reader::{process_csv, write_accounts};

fn main() {
    process_csv("basic.csv").and_then(|x| {
        write_accounts(x).and_then(|x| {
            print!("{}", x);
            Ok(())
        })
    }).unwrap();
}
