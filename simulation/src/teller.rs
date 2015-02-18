use std::sync::mpsc::{Sender, Receiver, channel};

use messages::*;

struct Teller {
  id: usize,
  buy_requests: Vec<TransactionRequest>,
  sell_requests: Vec<TransactionRequest>
}

pub fn start_teller(teller_id: usize, market_tx: Sender<MarketMessages>, teller_rx: Receiver<TellerMessages>) {
  let mut teller = Teller{id: teller_id, buy_requests: vec![], sell_requests: vec![]};

  loop {
    let message = teller_rx.recv().unwrap();
    match message {
      TellerMessages::BuyRequest(request) => {teller.buy_requests.push(request.clone());
                                              match make_buy_request(&mut teller) {
                                                Some(sell) => {market_tx.send(MarketMessages::MatchRequest(request, sell));}
                                                None => {} };
                                              println!("RECEIVED BUY REQUEST")},
      TellerMessages::SellRequest(request) => {teller.sell_requests.push(request.clone());
                                              match make_sell_request(&mut teller) {
                                                Some(buy) => {market_tx.send(MarketMessages::MatchRequest(buy, request));}
                                                None => {} }},
      TellerMessages::RevokeRequest(actor_id, transaction_id) => {revoke(actor_id, transaction_id, &mut teller);}
    }
  }
}

fn make_buy_request(teller: &mut Teller) -> Option<TransactionRequest> {
  let new_buy = teller.buy_requests[teller.buy_requests.len() -1].clone();
  let mut matching_sell: Option<TransactionRequest> = None;
  for i in 0..teller.sell_requests.len() {
    if new_buy.price >= teller.sell_requests[i].price {
       matching_sell = Some(teller.sell_requests[i].clone());
       teller.buy_requests.pop();
       teller.sell_requests.remove(i);
       break;
    }
  }
  matching_sell
}

fn make_sell_request(teller: &mut Teller) -> Option<TransactionRequest> {
    let new_sell = teller.sell_requests[teller.sell_requests.len() -1].clone();
    let mut matching_buy: Option<TransactionRequest> = None;
    for i in 0..teller.buy_requests.len() {
      if new_sell.price <= teller.buy_requests[i].price {
         matching_buy = Some(teller.buy_requests[i].clone());
         teller.sell_requests.pop();
         teller.buy_requests.remove(i);
         break;
      }
    }
    matching_buy
}

fn revoke(actor_id: usize, transaction_id: usize, teller: &mut Teller) {
  for i in 0..teller.buy_requests.len() {
    if actor_id == teller.buy_requests[i].actor_id && transaction_id == teller.buy_requests[i].transaction_id {
      teller.buy_requests.remove(i);
      return;
    }
  }
  for i in 0..teller.sell_requests.len() {
    if actor_id == teller.sell_requests[i].actor_id && transaction_id == teller.sell_requests[i].transaction_id {
      teller.sell_requests.remove(i);
      return;
    }
  }
}
