use std::collections::HashMap;
use std::sync::mpsc::{Sender, Receiver, channel};
use std::thread::Thread;
use std::cmp;

use messages::*;
use teller::*;

struct Market {
  id: usize,
  tellers: HashMap<usize, Sender<TellerMessages>>,
  actors: HashMap<usize, Sender<ActorMessages>>,
  active_transactions: Vec<(TransactionRequest, TransactionRequest)>,
  pending_transactions: Vec<(TransactionRequest, TransactionRequest)>,
  committed_actors: Vec<usize>,
}

//Called on a new thread
pub fn start_market(market_id: usize, market_tx: Sender<MarketMessages>, market_rx: Receiver<MarketMessages>) {
  //Create Market struct
  let mut market = Market {id:market_id,
                             tellers: HashMap::new(),
                             actors: HashMap::new(),
                             active_transactions: vec![],
                             committed_actors: vec![],
                             pending_transactions: vec![]};
  //TODO: Initialize Tellers
  for i in 0..1 {
    let (tx, rx): (Sender<TellerMessages>, Receiver<TellerMessages>) = channel();
    market.tellers.insert(i, tx);
    let market_tx_clone = market_tx.clone();
    Thread::spawn(move ||
      {start_teller(i, market_tx_clone, rx);});
  }

  //Start the receive loop
  loop {
    let message = market_rx.recv().unwrap();
    match message {
        MarketMessages::SellRequest(request) => {route(false, request, &market)},
        MarketMessages::BuyRequest(request) => {route(true, request, &market)},
        MarketMessages::Commit(actor_id) => {
          println!("Commit {}", actor_id);
          if has_active_transaction(&market, actor_id) {
            market.committed_actors.push(actor_id);
            let tup = get_active_transaction_involving(&market, actor_id);
            if contains(&market.committed_actors, tup.0.actor_id) && contains(&market.committed_actors, tup.1.actor_id) {
              //make the transaction complete. Tell the actors to commit;
              remove(&mut market.committed_actors, tup.0.actor_id);
              remove(&mut market.committed_actors, tup.1.actor_id);

              route_actor_message(&market, tup.0.actor_id, ActorMessages::CommitTransaction(tup.1.clone()));
              route_actor_message(&market, tup.1.actor_id, ActorMessages::CommitTransaction(tup.0.clone()));

              remove_active_transaction(&mut market, &tup);
              move_pending_to_active(&mut market, tup.0.actor_id, tup.1.actor_id);
            }
          }
        }
        MarketMessages::Cancel(actor_id) => {
          println!("Cancel {}", actor_id);
          if has_active_transaction(&market, actor_id) {
            let tup = get_active_transaction_involving(&market, actor_id);
            route_actor_message(&market, tup.0.actor_id, ActorMessages::AbortTransaction);
            route_actor_message(&market, tup.1.actor_id, ActorMessages::AbortTransaction);

            //remove it from active
            remove_active_transaction(&mut market, &tup);

            move_pending_to_active(&mut market, tup.0.actor_id, tup.1.actor_id);
          }
          },
        MarketMessages::RegisterActor(actor_id, actor_tx) => {
          let temp_clone = actor_tx.clone();
          market.actors.insert(actor_id, actor_tx);},
        MarketMessages::MatchRequest(mut buyer, mut seller) => {
          if has_active_transaction(&market, buyer.actor_id) || has_active_transaction(&market, seller.actor_id) {
            //add to pending transactions
            market.pending_transactions.push((buyer, seller));
          }
          else {
            activate_transactions(&mut market, buyer, seller);
          }
          },
    }
  }
}

fn activate_transactions(market: &mut Market, mut buyer: TransactionRequest, mut seller: TransactionRequest) {
  //add to active transactions and notify both.
  buyer.price = seller.price;
  let smaller_quantity = cmp::min(seller.quantity, buyer.quantity);
  buyer.quantity = smaller_quantity;
  seller.quantity = smaller_quantity;
  let amount_to_pay = buyer.quantity * buyer.price;
  let buyer_request = MoneyRequest {market_id: market.id, amount: amount_to_pay};
  let seller_request = StockRequest {market_id: market.id, stock_id: seller.stock_id, quantity: seller.quantity};

  route_actor_message(&market, buyer.actor_id, ActorMessages::MoneyRequest(buyer_request));
  route_actor_message(&market, seller.actor_id, ActorMessages::StockRequest(seller_request));
  market.active_transactions.push((buyer, seller));
}

fn move_pending_to_active(market: &mut Market, actor1: usize, actor2: usize) {
  //actor 1
  for i in 0..market.pending_transactions.len() {
    let pending_transaction = market.pending_transactions[i].clone();
    if (pending_transaction.0.actor_id == actor1) {
      if !has_active_transaction(&market, pending_transaction.1.actor_id) {
        activate_transactions(market, pending_transaction.0.clone(), pending_transaction.1.clone());
        remove(&mut market.pending_transactions, pending_transaction);
        break;
      }
    }
    if (pending_transaction.1.actor_id == actor1) {
      if !has_active_transaction(&market, pending_transaction.0.actor_id) {
        activate_transactions(market, pending_transaction.0.clone(), pending_transaction.1.clone());
        remove(&mut market.pending_transactions, pending_transaction);
        break;
      }
    }
  }

  //actor 2
  for i in 0..market.pending_transactions.len() {
    let pending_transaction = market.pending_transactions[i].clone();
    if (pending_transaction.0.actor_id == actor2) {
      if !has_active_transaction(&market, pending_transaction.1.actor_id) {
        activate_transactions(market, pending_transaction.0.clone(), pending_transaction.1.clone());
        remove(&mut market.pending_transactions, pending_transaction);
        break;
      }
    }
    if (pending_transaction.1.actor_id == actor2) {
      if !has_active_transaction(&market, pending_transaction.0.actor_id) {
        activate_transactions(market, pending_transaction.0.clone(), pending_transaction.1.clone());
        remove(&mut market.pending_transactions, pending_transaction);
        break;
      }
    }
  }
}

fn remove<T:PartialEq>(vec: &mut Vec<T>, element: T) {
  for i in 0..vec.len() {
    if vec[i] == element {
      vec.remove(i);
      break;
    }
  }
}

fn contains<T:PartialEq>(vec: &Vec<T>, element: T) -> bool {
  for test_element in vec.iter() {
    if element == *test_element {
      return true;
    }
  }
  false
}

fn remove_active_transaction(market: &mut Market, tup: &(TransactionRequest, TransactionRequest)) {
  for i in 0..market.active_transactions.len() {
    let test_active = market.active_transactions[i].clone();
    if  test_active.0 == tup.0 && test_active.1 == tup.1 {
       market.active_transactions.remove(i);
       break;
    }
  }
}

fn get_active_transaction_involving(market_ref : &Market, id: usize) -> (TransactionRequest, TransactionRequest) {
  for transaction_pair in market_ref.active_transactions.iter() {
    if transaction_pair.0.actor_id == id || transaction_pair.1.actor_id == id {
      return transaction_pair.clone();
    }
  }
  (TransactionRequest {transaction_id: 0, actor_id: 0, stock_id: 0, price: 0, quantity: 0}, TransactionRequest {transaction_id: 0, actor_id: 0, stock_id: 0, price: 0, quantity: 0})
}

fn has_active_transaction(market_ref : &Market, id: usize) -> bool {
  for transaction_pair in market_ref.active_transactions.iter() {
    if transaction_pair.0.actor_id == id || transaction_pair.1.actor_id == id {
      return true;
    }
  }
  false
}

fn route_actor_message(market: & Market, actor_id: usize, message: ActorMessages) {
  match market.actors.get(&actor_id) {
    Some(channel) => {channel.send(message);},
    None => {}
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
