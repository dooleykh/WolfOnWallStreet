use std::collections::HashMap;
use std::sync::mpsc::{Sender, Receiver, channel};
use std::sync::mpsc::TryRecvError;
use std::old_io::timer;
use std::time::Duration;

use messages::*;

struct Actor {
  id: usize,
  money: usize,
  stocks: HashMap<usize, usize>,
  pending_money: usize,
  pending_stock: (usize, usize), //Stock, quantity
  markets: HashMap<usize, Sender<MarketMessages>>
}

pub fn start_actor(actor_id: usize, existing_markets: HashMap<usize, Sender<MarketMessages>>) {
  println!("Starting Actor {}", actor_id);
  let mut actor = Actor { id: actor_id,
                          money: 100,
                          stocks: HashMap::new(),
                          pending_money: 0,
                          pending_stock: (0, 0),
                          markets: existing_markets};

  let (actor_tx, actor_rx): (Sender<usize>, Receiver<usize>) = channel();
  for (name, market_tx) in actor.markets.iter() {
    market_tx.send(MarketMessages::RegisterActor(actor.id, actor_tx.clone()));
  }

  loop {
    //Logic

    match actor_rx.try_recv() {
      Ok(id) => {println!("Actor {} received {}", actor.id, id);},
      Err(TryRecvError::Empty) => {timer::sleep(Duration::milliseconds(1000));},
      Err(TryRecvError::Disconnected) => {println!("ERROR: Actor {} disconnected", actor.id);}
    }
  }
}
