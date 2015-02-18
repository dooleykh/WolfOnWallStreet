use std::collections::HashMap;
use std::sync::mpsc::{Sender, Receiver};

use messages::*;

struct Market {
  tellers: HashMap<usize, Sender<TellerMessages>>,
  actors: HashMap<usize, Sender<usize>>, //TODO: change to ActorMessages after testing
  active_transactions: Vec<(TransactionRequest, TransactionRequest)>,
  pending_transactions: Vec<(TransactionRequest, TransactionRequest)>,
}

//Called on a new thread
pub fn start_market(market_rx: Receiver<MarketMessages>) {
  //Create Market struct
  let mut exchange = Market {tellers: HashMap::new(),
                             actors: HashMap::new(),
                             active_transactions: vec![],
                             pending_transactions: vec![]};
  //TODO: Initialize Tellers

  //Start the receive loop
  loop {
    let message = market_rx.recv().unwrap();
    match message {
        MarketMessages::SellRequest(request) => {},
        MarketMessages::BuyRequest(request) => {},
        MarketMessages::Commit(actor_id) => {},
        MarketMessages::Cancel(actor_id) => {},
        MarketMessages::RegisterActor(actor_id, actor_tx) => {
          exchange.actors.insert(actor_id, actor_tx);
          for (id, actor_tx) in exchange.actors.iter() {
            actor_tx.send(*id);
          };}
        MarketMessages::MatchRequest(buyer, sender) => {},
    }
  }
}
