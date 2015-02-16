use std::collections::HashMap;
use std::sync::mpsc::{Sender, Receiver, channel};
use messages::*;

struct Actor {
  id: usize,
  money: usize,
  stocks: HashMap<String, usize>,
  pending_money: usize,
  pending_stock: (String, usize),
  markets: HashMap<String, Sender<MarketMessages>>
}

pub fn start_actor(actor_id: usize, existing_markets: HashMap<String, Sender<MarketMessages>>) {
  println!("Starting Actor {}", actor_id);
  let mut actor = Actor { id: actor_id,
                          money: 100,
                          stocks: HashMap::new(),
                          pending_money: 0,
                          pending_stock: ("".to_string(), 0),
                          markets: existing_markets};

  let (actor_tx, actor_rx): (Sender<usize>, Receiver<usize>) = channel();
  for (name, market_tx) in actor.markets.iter() {
    market_tx.send(MarketMessages::RegisterActor(actor.id, actor_tx.clone()));
  }

  loop {
    let message = actor_rx.recv().unwrap();
    println!("Actor {} received {}", actor.id, message);
  };
}
