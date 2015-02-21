use std::collections::HashMap;
use std::sync::mpsc::{Sender, Receiver, channel};
use std::thread::Thread;
use std::old_io::timer;
use std::time::Duration;

pub mod messages;
use messages::*;
pub mod market;
use market::*;
pub mod actor;
use actor::*;
pub mod teller;
use teller::*;
pub mod corporate_actor;
use corporate_actor::*;

fn main() {
  //tx: clone for actors        rx: owned by market
  let (tx_market, rx_market): (Sender<MarketMessages>, Receiver<MarketMessages>) = channel();

  //TODO: with spawning multiple markets make this a for loop.
  let tx_market_clone = tx_market.clone();
  Thread::spawn(move || {
    market::start_market(0, tx_market_clone, rx_market);});

  let mut markets = HashMap::new();
  markets.insert(0, tx_market.clone());

  let standard_actor_count = 5;
  let corporate_actor_count = 1;
  //TODO make more stocks and add history so actors can query on it.
  for id in 0..standard_actor_count {
    let m = markets.clone();
    Thread::spawn(move || {actor::start_actor(id, m);});
  }
  for id in standard_actor_count..standard_actor_count+corporate_actor_count {
    let m = markets.clone();
    Thread::spawn(move || {corporate_actor::start_corporate_actor(id, m, 0, 100);});//start a corporate actor with 100 of stock 0
  }

  timer::sleep(Duration::milliseconds(10000));
}
