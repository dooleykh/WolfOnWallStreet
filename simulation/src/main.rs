fn main() {
    println!("Hello, world!");
}

// Messages from Actors to a Market
enum MarketMessages {
  SellRequest(TransactionRequest),
  BuyRequest(TransactionRequest),
  Commit(usize), //Id of the actor
  Cancel(usize), //Id of the actor
}

// Messages from a Market to an Actor
enum ActorMessages {
  StockRequest(StockRequest),
  MoneyRequest(usize), //The amount of money needed to buy the stock(s)
  CommitTransaction(TransactionRequest), //The information related to the transaction
  AbortTransaction,

}

struct TransactionRequest {
  id: usize,
  stock_id: usize,
  price: usize,
  quantity: usize
}

struct StockRequest {
  stock_id: usize,
  quantity: usize,
}
