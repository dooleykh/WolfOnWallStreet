use std::collections::HashMap;
use std::sync::mpsc::{Sender, Receiver, channel};
use std::sync::mpsc::TryRecvError;
use std::sync::{Arc, Mutex};
use std::old_io::timer;
use std::time::Duration;

use messages::{ActorMessages, TransactionRequest, MarketMessages, MarketHistory};
use messages::ActorMessages::{StockRequest, MoneyRequest, CommitTransaction, AbortTransaction, History, Time};
use messages::MarketMessages::{SellRequest, Commit, Cancel, RegisterActor, MatchRequest};
use actor::Actor;

// Smarter actor
// (monitors price last sold at and put a sell request if any stocks are above their purchase price)


pub fn start_smarter_actor(actor_id: usize, existing_markets: HashMap<usize, Sender<MarketMessages>>, actor_tx: Sender<ActorMessages>, actor_rx: Receiver<ActorMessages>) {
  println!("Starting Smarter_Actor {}", actor_id);
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
  let mut bought_stocks : HashMap<usize, HashMap<usize,(usize,usize)>> = HashMap::new();

  for (_, market_tx) in actor.markets.iter() {
    market_tx.send(RegisterActor(actor.id, actor_tx.clone())).unwrap();
  }

  loop {
    //Logic
    let mark_clone = actor.markets.clone();
    let stock_clone = actor.stocks.clone();

    if(init_history){
      let hist = actor.history.lock();

      // For each stock in history
      for stock in hist.stocks.iter(){
        // if we own the stock and the last sold transaction was above my bought price submit a sell
        
        // if we don't own the stock submit a buy for the last sold transaction
      }

      println!("History has length {} for smarter_actor.",hist.stocks.len());
    }

    match actor_rx.try_recv() {
      Ok(message) => {
          match message {
            StockRequest(stock_request) => {
              println!("Started Stock Request in smarter_actor {}", actor.id);
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
              print_status(&actor);
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
              print_status(&actor);
              },
            CommitTransaction(commit_transaction_request) => {
              //if we have money pending, then look up the stock id and add that quantity purchased.
              //remove the pending money
              if actor.pending_money > 0 {
                let price_per_unit = commit_transaction_request.price;
                let units = commit_transaction_request.quantity;
                let leftover_money = actor.pending_money - price_per_unit * units;

                //make a function for adding stock.
                add_stock(&mut actor, (commit_transaction_request.stock_id, units));
                actor.money = actor.money + leftover_money;

                actor.pending_money = 0;
              }

              //if we have stock pending, look up the quantity purchased and add the money.
              //remove the pending stock
              if actor.pending_stock.1 > 0 {
                let money = commit_transaction_request.price * commit_transaction_request.quantity;
                let restore_stock = (commit_transaction_request.stock_id, actor.pending_stock.1 - commit_transaction_request.quantity);
                if restore_stock.1 > 0 {
                  add_stock(&mut actor, restore_stock);
                }
                actor.money = actor.money + money;
                actor.pending_stock = (0,0);
              }
              println!("After a committed transaction in smarter_actor {} ", actor.id);
              print_status(&actor);
              },
            AbortTransaction => {
              //move pending stock back into stocks.
              if actor.pending_stock.1 != 0 {
                let pending_stock_clone = actor.pending_stock.clone();
                add_stock(&mut actor, pending_stock_clone);
                //now that we have moved it. Clear out the pending stock.
                actor.pending_stock = (0,0); //setting the quantity to zero clears it.
              }
              for (stock_id, quantity) in actor.stocks.iter() {
                println!("After aborting the transaction, smarter_actor {} now has StockId: {} Quantity: {}", actor.id, *stock_id, *quantity);
              }

              //move pending money back into money.
              if actor.pending_money > 0 {
                actor.money = actor.money + actor.pending_money;
                actor.pending_money = 0;
              }
              println!("After aborting the transaction, smarter_actor {} now has {} money.", actor.id, actor.money);
            },
            History(history) => {
              println!("Smarter_actor {} received history {}", actor.id, *(history.lock().unwrap()));
              actor.history = history;
              init_history = true;
              },
            Time(current, max) => {
              println!("Smarter_actor {} received time {}", actor.id, current);
            }
          }
        },
      Err(TryRecvError::Empty) => {timer::sleep(Duration::milliseconds(10));},
      Err(TryRecvError::Disconnected) => {println!("ERROR: Smarter_actor {} disconnected", actor.id);}
    }
  }
}

fn add_stock(actor: &mut Actor, stock_to_add: (usize, usize)) {
  let stock_clone = actor.stocks.clone();
  let held_stock = stock_clone.get(&stock_to_add.0);
  match held_stock {
    Some(stock_count) => {
        //we have some stock. We need to add to our reserve.
        actor.stocks.insert(stock_to_add.0, stock_to_add.1 + *stock_count);
      },
    None => {
      //we don't have any stock left. Just add it back.
      actor.stocks.insert(stock_to_add.0, stock_to_add.1);
    }
  }
}

fn remove_stock(actor: &mut Actor, stock_to_remove: (usize, usize)) {
  let stock_clone = actor.stocks.clone();
  let held_stock = stock_clone.get(&stock_to_remove.0);
  match held_stock {
    Some(stock_count) => {
        //we have some stock. We need to add to our reserve.
        actor.stocks.insert(stock_to_remove.0, *stock_count - stock_to_remove.1);
      },
    None => {} //TODO Should we error handle here?
  }
}

fn print_status(actor: &Actor) {
  for (stock_id, quantity) in (*actor).stocks.iter() {
    println!("Actor {} now has StockId: {} Quantity: {}", (*actor).id, *stock_id, *quantity);
  }
  println!("Actor {} has {} money.", (*actor).id, (*actor).money);
}

fn has_pending_transaction(actor: &Actor) -> bool {
  (*actor).pending_money > 0 || (*actor).pending_stock.1 > 0
}
