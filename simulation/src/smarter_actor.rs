use std::collections::HashMap;
use std::sync::mpsc::{Sender, Receiver};
use std::sync::mpsc::TryRecvError;
use std::sync::{Arc, Mutex};
use std::old_io::timer;
use std::time::Duration;
use std::cmp::max;

use messages::{ActorMessages, TransactionRequest, MarketMessages, MarketHistory};
use messages::ActorMessages::{StockRequest, MoneyRequest, CommitTransaction, AbortTransaction, History, Time, ReceiveActivityCount, Stop};
use messages::MarketMessages::{BuyRequest, SellRequest, Commit, Cancel, RegisterActor};
use actor::Actor;
use actor::{add_stock, remove_stock, status};

// Smarter actor
// (monitors price last sold at and put a sell request if any stocks are above their purchase price)


pub fn start_smarter_actor(actor_id: usize, existing_markets: HashMap<usize, Sender<MarketMessages>>, actor_tx: Sender<ActorMessages>, actor_rx: Receiver<ActorMessages>) {
  println!("Starting Smarter_Actor {}", actor_id);
  let mut stop_flag = false;
  let mut init_history = false;
  let mut actor = Actor { id: actor_id,
                          money: 100,
                          stocks: HashMap::new(),
                          pending_money: 0,
                          pending_stock: (0, 0),
                          markets: existing_markets,
                          history: Arc::new(Mutex::new(MarketHistory {history: HashMap::new(), stocks: vec![]}))
                          };
  // Stocks = HashMap<market_id, HashMap<stock_id, (price,quantity)>>
  let mut buy_requests : HashMap<usize, HashMap<usize,(usize,usize)>> = HashMap::new();
  let mut unique_id : usize = 0;

  for (_, market_tx) in actor.markets.iter() {
    market_tx.send(RegisterActor(actor.id, actor_tx.clone())).unwrap();
  }

  loop {
    if stop_flag {
      timer::sleep(Duration::milliseconds(1000));
      continue;
    }
    //Logic
    let mark_clone = actor.markets.clone();
    let stock_clone = actor.stocks.clone();

    if init_history {
      let hist = actor.history.lock().unwrap();

      // For each stock in history
      for stock in hist.stocks.iter(){
        match hist.last_transaction_for_stock(*stock) {
          Some((buyer, seller)) => {
            match buy_requests.clone().get(&0){
              Some(stock_requests) => {
                match stock_requests.get(stock){
                  Some(&(request_price,request_quantity)) => {
                    // If it's price is above ours request a sell
                    if buyer.price > request_price {
                      let trans : TransactionRequest  = TransactionRequest   {  transaction_id: unique_id
                                                                              , actor_id:actor_id
                                                                              , stock_id: stock.clone()
                                                                              , price:buyer.price
                                                                              , quantity:request_quantity
                                                                              };
                      send_message(0,&mark_clone,SellRequest(trans));
                      unique_id = unique_id + 1;
                    }
                  },
                  None => {
                    // Otherwise lets try to buy some stock to sell later
                    let request_quantity = (actor.money/10)/max(seller.price, 1);
                    let trans : TransactionRequest  = TransactionRequest{   transaction_id: unique_id
                                                                          , actor_id:actor_id
                                                                          , stock_id: stock.clone()
                                                                          , price:seller.price
                                                                          , quantity:request_quantity
                                                                        };
                    send_message(0,&mark_clone,BuyRequest(trans));
                    unique_id = unique_id + 1;
                  }
                }
              },
              None =>{
                buy_requests.insert(0,HashMap::new());
                } // Market didn't exist?

            }
          },
          None => {}
        }
      }
    }

    match actor_rx.try_recv() {
      Ok(message) => {
          match message {
            StockRequest(stock_request) => {
              let market_tx;
              let tx_text = mark_clone.get(&stock_request.market_id);
              match tx_text {
                Some(tx) => {
                  market_tx = tx;
                },
                None => {
                  return; //HOW DID WE GET A MISSING MARKET?
                }
              }
              if has_pending_transaction(&actor) {
                market_tx.send(Cancel(actor.id)).unwrap();
              }
              else {
                //if we have the stock. set it aside.
                let stock_id = stock_request.stock_id;
                let quantity = stock_request.quantity;
                let stock = stock_clone.get(&stock_id);
                match stock {
                  Some(owned_quantity) => {
                    if *owned_quantity >= quantity {
                      remove_stock(&mut actor, (stock_id, quantity));
                      actor.pending_stock = (stock_id, quantity);
                      market_tx.send(Commit(actor.id)).unwrap();
                    }
                    else {
                      market_tx.send(Cancel(actor.id)).unwrap();
                    }
                  },
                  None => {
                    market_tx.send(Cancel(actor.id)).unwrap();
                  }
                }
              }
              // print_status(&actor);
              },
            MoneyRequest(money_request) => {
              let market_tx;
              let tx_text = actor.markets.get(&money_request.market_id);
              match tx_text {
                Some(tx) => {
                  market_tx = tx;
                },
                None => {
                  return; //HOW DID WE GET A MISSING MARKET?
                }
              }
              if has_pending_transaction(&actor) {
                market_tx.send(Cancel(actor.id)).unwrap();
              }
              else {
                //if we have the money. set it aside.
                if actor.money >= money_request.amount {
                  actor.money = actor.money - money_request.amount;
                  actor.pending_money = money_request.amount;
                  market_tx.send(Commit(actor.id)).unwrap();
                }
                else {
                  market_tx.send(Cancel(actor.id)).unwrap();
                }
              }
              // print_status(&actor);
              },
            CommitTransaction(commit_transaction_request) => {
              //if we have money pending, then look up the stock id and add that quantity purchased.
              //remove the pending money
              if actor.pending_money > 0 {
                let units = commit_transaction_request.quantity;
                let leftover_money = actor.pending_money - commit_transaction_request.price;

                //make a function for adding stock.
                add_stock(&mut actor, (commit_transaction_request.stock_id, units));
                actor.money = actor.money + leftover_money;

                actor.pending_money = 0;
              }

              //if we have stock pending, look up the quantity purchased and add the money.
              //remove the pending stock
              if actor.pending_stock.1 > 0 {
                let money = commit_transaction_request.price;
                let restore_stock = (commit_transaction_request.stock_id, actor.pending_stock.1 - commit_transaction_request.quantity);
                if restore_stock.1 > 0 {
                  add_stock(&mut actor, restore_stock);
                }
                actor.money = actor.money + money;
                actor.pending_stock = (0,0);
              }
            },
            AbortTransaction => {
              //move pending stock back into stocks.
              if actor.pending_stock.1 != 0 {
                let pending_stock_clone = actor.pending_stock.clone();
                add_stock(&mut actor, pending_stock_clone);
                //now that we have moved it. Clear out the pending stock.
                actor.pending_stock = (0,0); //setting the quantity to zero clears it.
              }

              //move pending money back into money.
              if actor.pending_money > 0 {
                actor.money = actor.money + actor.pending_money;
                actor.pending_money = 0;
              }
            },
            History(history) => {
              actor.history = history;
              init_history = true;
              },
            Time(_, _) => {},
            ReceiveActivityCount(_,_,_) => {},
            Stop(main_channel) => {
              main_channel.send((actor.id, "(Smarter Actor) ".to_string() + status(&actor).as_slice())).unwrap();
              stop_flag = true;
            }
          }
        },
      Err(TryRecvError::Empty) => {timer::sleep(Duration::milliseconds(10));},
      Err(TryRecvError::Disconnected) => {println!("ERROR: Smarter_actor {} disconnected", actor.id);}
    }
  }
}

fn send_message(market_id : usize, markets: &HashMap<usize, Sender<MarketMessages>>, message: MarketMessages){
  match markets.get(&market_id){
    Some(market) => {
      market.send(message).unwrap();
      },
    None => {}
  }
}

fn has_pending_transaction(actor: &Actor) -> bool {
  (*actor).pending_money > 0 || (*actor).pending_stock.1 > 0
}
