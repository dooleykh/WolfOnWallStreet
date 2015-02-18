use std::collections::HashMap;
use std::sync::mpsc::{Sender, Receiver, channel};
use std::thread::Thread;

use messages::*;
use teller::*;

struct Market {
  tellers: HashMap<usize, Sender<TellerMessages>>,
  actors: HashMap<usize, Sender<ActorMessages>>,
  active_transactions: Vec<(TransactionRequest, TransactionRequest)>,
  pending_transactions: Vec<(TransactionRequest, TransactionRequest)>,
}

//Called on a new thread
pub fn start_market(market_tx: Sender<MarketMessages>, market_rx: Receiver<MarketMessages>) {
  //Create Market struct
  let mut exchange = Market {tellers: HashMap::new(),
                             actors: HashMap::new(),
                             active_transactions: vec![],
                             pending_transactions: vec![]};
  //TODO: Initialize Tellers
  for i in 0..1 {
    let (tx, rx): (Sender<TellerMessages>, Receiver<TellerMessages>) = channel();
    exchange.tellers.insert(i, tx);
    let market_tx_clone = market_tx.clone();
    Thread::spawn(move ||
      {start_teller(i, market_tx_clone, rx);});
  }

  //Start the receive loop
  loop {
    let message = market_rx.recv().unwrap();
    match message {
        MarketMessages::SellRequest(request) => {route(false, request, &exchange)},
        MarketMessages::BuyRequest(request) => {route(true, request, &exchange)},
        MarketMessages::Commit(actor_id) => {},
        MarketMessages::Cancel(actor_id) => {},
        MarketMessages::RegisterActor(actor_id, actor_tx) => {
          exchange.actors.insert(actor_id, actor_tx);},
        MarketMessages::MatchRequest(buyer, sender) => {},
    }
  }
}

fn route(buying: bool, transaction: TransactionRequest, market: & Market) {
  let tx;
  match market.tellers.get(&transaction.stock_id) {
    Some(channel) => {tx = channel;},
    None => {return;}
  }
  if buying {
    tx.send(TellerMessages::BuyRequest(transaction));
  }
  else {
    tx.send(TellerMessages::SellRequest(transaction));
  };
}
