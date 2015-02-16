use std::sync::mpsc::{Sender, Receiver};

// Messages to a Market
pub enum MarketMessages {
  SellRequest(TransactionRequest),
  BuyRequest(TransactionRequest),
  Commit(usize), //Id of the actor
  Cancel(usize), //Id of the actor
  RegisterActor(usize, Sender<usize>), //Actor's id, transmit channel

  MatchRequest(TransactionRequest, TransactionRequest) // (Buyer's Request, Seller's Request)
}

// Messages from a Market to an Actor
pub enum ActorMessages {
  StockRequest(StockRequest),
  MoneyRequest(usize), //The amount of money needed to buy the stock(s)
  CommitTransaction(TransactionRequest), //The information related to the transaction
  AbortTransaction,
}

// Messages from a Market to a Teller
pub enum TellerMessages {
  SellRequest(TransactionRequest),
  BuyRequest(TransactionRequest),
}

pub struct TransactionRequest {
  pub actor_id: usize, //Id of Actor initiating the request
  pub stock_id: usize,
  pub price: usize,
  pub quantity: usize
}

pub struct StockRequest {
  pub stock_id: usize,
  pub quantity: usize,
}
