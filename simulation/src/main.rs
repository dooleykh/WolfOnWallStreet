fn main() {
    println!("Hello, world!");
}

// Messages to a Market
enum MarketMessages {
  SellRequest(TransactionRequest),
  BuyRequest(TransactionRequest),
  Commit(usize), //Id of the actor
  Cancel(usize), //Id of the actor

  MatchRequest(TransactionRequest, TransactionRequest) // (Buyer's Request, Seller's Request)
}

// Messages from a Market to an Actor
enum ActorMessages {
  StockRequest(StockRequest),
  MoneyRequest(usize), //The amount of money needed to buy the stock(s)
  CommitTransaction(TransactionRequest), //The information related to the transaction
  AbortTransaction,
}

// Messages from a Market to a Teller
enum TellerMessages {
  SellRequest(TransactionRequest),
  BuyRequest(TransactionRequest),
}

struct TransactionRequest {
  actor_id: usize, //Id of Actor initiating the request
  stock_id: usize,
  price: usize,
  quantity: usize
}

struct StockRequest {
  stock_id: usize,
  quantity: usize,
}
