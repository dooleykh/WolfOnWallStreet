pub mod messages;
use messages::*;

fn main() {
  let sr = StockRequest {stock_id: 0, quantity: 1};
  println!("ID: {}, Quantity: {}", sr.stock_id, sr.quantity);
}
