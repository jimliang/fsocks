extern crate libc;
extern crate lru_cache;
extern crate tokio;
#[macro_use]
extern crate serde_derive;
extern crate clap;
extern crate serde;
extern crate serde_json;

#[macro_use]
mod util;
mod config;
mod context;
mod socks5;
mod tcp;

use clap::{App, Arg};
use config::Config;
use context::Context;
use tcp::TcpConnect;
use tokio::net::TcpListener;
use tokio::prelude::*;

fn main() {
    let matches = App::new("transparent redirector of any TCP connection")
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .arg(
            Arg::with_name("config")
                .short("c")
                .long("config")
                .takes_value(true),
        )
        .get_matches();
    let config = matches.value_of("config").unwrap_or("config.json");
    let file = ::std::fs::File::open(config).expect("config file not found");
    let config: Config = serde_json::from_reader(file).expect("parse config file error");
    run(config)
}

fn run(config: Config) {
    let context = Context::new(config);
    let listener = TcpListener::bind(context.local()).expect("unable to bind TCP listener");
    println!("listening tcp on {}", context.local().port());
    let server = listener
        .incoming()
        .map_err(|e| eprintln!("accept failed = {:?}", e))
        .for_each(move |sock| {
            let fut = TcpConnect::connect(sock, context.clone())
                .map(|(a, b)| println!("complete {} {}", a, b))
                .map_err(|e| eprintln!("handle connection failed = {:?}", e));
            tokio::spawn(fut)
        });
    tokio::run(server);
}
