use std::sync::mpsc::{Sender, Receiver};
use std::sync::{Arc, Mutex};
use std::fmt;

// Messages to a Market
pub enum MarketMessages {
  SellRequest(TransactionRequest),
  BuyRequest(TransactionRequest),
  Commit(usize), //Id of the actor
  Cancel(usize), //Id of the actor
  RegisterActor(usize, Sender<ActorMessages>), //Actor's id, transmit channel

  MatchRequest(TransactionRequest, TransactionRequest) // (Buyer's Request, Seller's Request)
}

// Messages from a Market to an Actor
pub enum ActorMessages {
  StockRequest(StockRequest),
  MoneyRequest(MoneyRequest), //The amount of money needed to buy the stock(s)
  CommitTransaction(TransactionRequest), //The information related to the transaction
  AbortTransaction,
  History(Arc<Mutex<MarketHistory>>)
}

// Messages from a Market to a Teller
pub enum TellerMessages {
  SellRequest(TransactionRequest),
  BuyRequest(TransactionRequest),
  RevokeRequest(usize, usize) //actor_id, transaction_id (unique to a single actor)
}

#[derive(Clone, PartialEq)]
pub struct TransactionRequest {
  pub transaction_id: usize,
  pub actor_id: usize, //Id of Actor initiating the request
  pub stock_id: usize,
  pub price: usize,
  pub quantity: usize
}

#[derive(Clone)]
pub struct StockRequest {
  pub market_id: usize,
  pub stock_id: usize,
  pub quantity: usize,
}

#[derive(Clone)]
pub struct MoneyRequest {
  pub market_id: usize,
  pub amount: usize,
}

pub struct MarketHistory {
  pub history: Vec<(TransactionRequest, TransactionRequest)>
}

impl fmt::Display for MarketHistory {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "History length is {}", self.history.len())
  }
}

impl MarketHistory {
  pub fn len(&self) -> usize {
    self.history.len()
  }
}
