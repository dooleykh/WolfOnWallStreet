#![feature(io, rand, std_misc)]
#![allow(deprecated)]
use std::collections::HashMap;
use std::sync::mpsc::{Sender, Receiver, channel};
use std::thread::Thread;
use std::old_io::timer;
use std::time::Duration;

pub mod messages;
pub mod market;
pub mod actor;
pub mod teller;
pub mod corporate_actor;
pub mod smarter_actor;
pub mod scripted_actor;
pub mod random_actor;
pub mod dummy_actor_1;

use messages::*;
use market::*;
use actor::*;
use corporate_actor::*;
use smarter_actor::*;
use scripted_actor::*;
use random_actor::*;
use dummy_actor_1::*;

fn main() {
  //tx: clone for actors        rx: owned by market
  let (tx_market, rx_market): (Sender<MarketMessages>, Receiver<MarketMessages>) = channel();

  let standard_actor_count = 5;
  let corporate_actor_count = 3;
  let scripted_actor_count = 2;
  let smarter_actor_count = 5;
  let random_actor_count = 5;
  let dummy_actor_1_count = 4;

  //TODO: with spawning multiple markets make this a for loop.
  let tx_market_clone = tx_market.clone();
  Thread::spawn(move || {
    market::start_market(0, tx_market_clone, rx_market, corporate_actor_count);});

  let mut markets = HashMap::new();
  markets.insert(0, tx_market.clone());

  let mut actors_with_timers = vec![];


  let mut current_id = 0;
  //TODO make more stocks and add history so actors can query on it.
  for _ in 0..standard_actor_count {
    let m = markets.clone();
    let (actor_tx, actor_rx): (Sender<ActorMessages>, Receiver<ActorMessages>) = channel();
    actors_with_timers.push(actor_tx.clone());
    Thread::spawn(move || {start_actor(current_id, m, actor_tx, actor_rx);});
    current_id += 1;
  }

  for id in 0..corporate_actor_count {
    let m = markets.clone();
    let (actor_tx, actor_rx): (Sender<ActorMessages>, Receiver<ActorMessages>) = channel();
    actors_with_timers.push(actor_tx.clone());
    Thread::spawn(move || {start_corporate_actor(current_id, m, id, 100, actor_tx, actor_rx);});//start a corporate actor with 100 of stock 0
    current_id += 1;
  }

  for _ in 0..scripted_actor_count {
    let m = markets.clone();
    let (actor_tx, actor_rx): (Sender<ActorMessages>, Receiver<ActorMessages>) = channel();
    actors_with_timers.push(actor_tx.clone());
    Thread::spawn(move || {start_scripted_actor(current_id, m, actor_tx, actor_rx);});
    current_id += 1;
  }

  for _ in 0..smarter_actor_count{
    let m = markets.clone();
    let (actor_tx, actor_rx): (Sender<ActorMessages>, Receiver<ActorMessages>) = channel();
    actors_with_timers.push(actor_tx.clone());
    Thread::spawn(move || {start_smarter_actor(current_id, m, actor_tx, actor_rx);});
    current_id += 1;
  }

  for _ in 0..random_actor_count{
    let m = markets.clone();
    let (actor_tx, actor_rx): (Sender<ActorMessages>, Receiver<ActorMessages>) = channel();
    actors_with_timers.push(actor_tx.clone());
    Thread::spawn(move || {start_random_actor(current_id, m, actor_tx, actor_rx);});
    current_id += 1;
  }

  for _ in 0..dummy_actor_1_count{
    let m = markets.clone();
    let (actor_tx, actor_rx): (Sender<ActorMessages>, Receiver<ActorMessages>) = channel();
    actors_with_timers.push(actor_tx.clone());
    Thread::spawn(move || {start_dummy_actor_1(current_id, m, actor_tx, actor_rx);});
    current_id += 1;
  }

  let tick = 100;
  for t in 0..248 {
    for tx in actors_with_timers.iter() {
      tx.send(ActorMessages::Time(t * tick, 247 * tick)).unwrap();
    }
    timer::sleep(Duration::milliseconds(tick as i64));
  }

  for tx in actors_with_timers.iter() {
    tx.send(ActorMessages::Stop).unwrap();
  }

  timer::sleep(Duration::milliseconds(1000));
}
