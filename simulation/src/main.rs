use std::collections::HashMap;
use std::sync::mpsc::{Sender, Receiver, channel};
use std::thread::Thread;

pub mod messages;
use messages::*;
pub mod market;
use market::*;
pub mod actor;
use actor::*;

fn main() {
  let sr = StockRequest {stock_id: 0, quantity: 1};
  println!("ID: {}, Quantity: {}", sr.stock_id, sr.quantity);

  //tx: clone for actors        rx: owned by market
  let (tx_market, rx_market): (Sender<MarketMessages>, Receiver<MarketMessages>) = channel();

  Thread::spawn(move || {
    market::start_market(rx_market);});

  let mut markets = HashMap::new();
  markets.insert("BSE".to_string(), tx_market.clone());

  for id in 0..10 {
    let m = markets.clone();
    Thread::spawn(move || {actor::start_actor(id, m);});
  }
  loop {};
}
