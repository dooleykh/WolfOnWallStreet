use std::collections::HashMap;
use std::sync::mpsc::{Sender, Receiver};
use std::sync::mpsc::TryRecvError;
use std::sync::{Arc, Mutex};
use std::old_io::timer;
use std::time::Duration;

use messages::{MarketMessages, MarketHistory, ActorMessages, TransactionRequest};
use messages::ActorMessages::{StockRequest, MoneyRequest, CommitTransaction, AbortTransaction, History, Time, ReceiveActivityCount};
use messages::MarketMessages::{BuyRequest, Commit, Cancel, RegisterActor, SellRequest};
use actor::Actor;

pub fn start_dummy_actor_1(actor_id: usize, existing_markets: HashMap<usize, Sender<MarketMessages>>, actor_tx: Sender<ActorMessages>, actor_rx: Receiver<ActorMessages>) {
  println!("Starting Dummy_Actor_1 {}", actor_id);
  let mut actor = Actor { id: actor_id,
                          money: 100,
                          stocks: HashMap::new(),
                          pending_money: 0,
                          pending_stock: (0, 0),
                          markets: existing_markets,
                          history: Arc::new(Mutex::new(MarketHistory {history: HashMap::new(), stocks: vec![]}))};

  //Register the actor with every market
  for (_, market_tx) in actor.markets.iter() {
    market_tx.send(RegisterActor(actor.id, actor_tx.clone())).unwrap();
  }

  //Number of stocks available to buy
  let numStocks = actor.stocks.len();
  let mut init_history = false;
  let mut stop_flag = false;
  let mut toSellPrices = HashMap::new();
  let mut stock_id_incr = 0;


  loop {
    if stop_flag {
      timer::sleep(Duration::milliseconds(1000));
      continue;
    }

    //Logic
    let mark_clone = actor.markets.clone();
    let stock_clone = actor.stocks.clone();

    //buying and selling decisions
    ////////////////////////////////////////////////////////////////////
    if init_history {
      //Get variables for the actor's stocks
      let local_stocks;
      {
        local_stocks = actor.history.lock().unwrap().stocks.clone();
      }

      //Iterate through the actor's stocks
      for stock in local_stocks.iter() {
        match actor.stocks.get(stock) {
          //If the actor has some of a stock
          Some(count) => {
            //And he has not yet sent out a sell request
            if toSellPrices.contains_key(stock) {
              //Get the price he should sell it at (remove from HashMap)
              let sell_price = toSellPrices.remove(stock);
              match sell_price {
                Some(price) => {
                  for (_, market_tx) in actor.markets.iter() {
                    //Send out a sell request to sell it
                    let t = TransactionRequest{actor_id: actor.id, transaction_id: stock_id_incr, stock_id: *stock, price: price, quantity: 1};
                    market_tx.send(SellRequest(t)).unwrap();
                    stock_id_incr = stock_id_incr + 1;
                  }
                },
                None => {}
              }
            }
          }
          //If the actor has none of a stock
          None => {
            if !toSellPrices.contains_key(stock) {
              //Make the price he should buy it at the most recently bought price
              let buy_price = actor.history.lock().unwrap().last_sold_price(*stock);
              match buy_price {
                //If the stock was last bought at a price
                Some(price) => {
                  //If the actor can afford to buy it
                  if actor.money > price {
                    for (_, market_tx) in actor.markets.iter() {
                      let t = TransactionRequest{actor_id: actor.id, transaction_id: stock_id_incr, stock_id: *stock, price: price, quantity: 1};
                      market_tx.send(BuyRequest(t)).unwrap();
                      stock_id_incr = stock_id_incr + 1;
                      toSellPrices.insert(*stock, price * 2);
                    }
                  }
                },
                //If the stock has not been bought yet
                None => {}
              }
            }
          }
        }
      }

    }
    /////////////////////////////////////////////////////////////////////

    match actor_rx.try_recv() {
      Ok(message) => {
          match message {
            StockRequest(stock_request) => {
              //println!("Started Stock Request in dummy_actor_1 {}", actor.id);
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
              //print_status(&actor);
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
              //print_status(&actor);
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
              //println!("After a committed transaction in actor {} ", actor.id);
              //print_status(&actor);
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
                //println!("After aborting the transaction, actor {} now has StockId: {} Quantity: {}", actor.id, *stock_id, *quantity);
              }

              //move pending money back into money.
              if actor.pending_money > 0 {
                actor.money = actor.money + actor.pending_money;
                actor.pending_money = 0;
              }
              //println!("After aborting the transaction, actor {} now has {} money.", actor.id, actor.money);
            },
            History(history) => {
              //println!("Actor {} received history {}", actor.id, *(history.lock().unwrap()));
              actor.history = history;
              init_history = true},
            Time(_, _) => {},
            ReceiveActivityCount(_,_,_) => {},
            Stop => {
              print_status(&actor);
              stop_flag = true;
            }
          }
        },
      Err(TryRecvError::Empty) => {timer::sleep(Duration::milliseconds(1));},
      Err(TryRecvError::Disconnected) => {println!("ERROR: Actor {} disconnected", actor.id);}
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
    println!("Dummy Actor_1 {} now has StockId: {} Quantity: {}", (*actor).id, *stock_id, *quantity);
  }
  println!("Dummy Actor_1 {} has {} money.", (*actor).id, (*actor).money);
}

fn has_pending_transaction(actor: &Actor) -> bool {
  (*actor).pending_money > 0 || (*actor).pending_stock.1 > 0
}
