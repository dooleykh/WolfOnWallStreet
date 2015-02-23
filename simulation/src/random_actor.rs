use std::collections::HashMap;
use std::rand;
use std::rand::Rng;
use std::sync::mpsc::{Sender, Receiver};
use std::sync::mpsc::TryRecvError;
use std::sync::{Arc, Mutex};
use std::old_io::timer;
use std::time::Duration;

use messages::{MarketMessages, MarketHistory, ActorMessages, TransactionRequest};
use messages::ActorMessages::{StockRequest, MoneyRequest, CommitTransaction, AbortTransaction, History, Time, ReceiveActivityCount, Stop};
use messages::MarketMessages::{BuyRequest, Commit, Cancel, RegisterActor, SellRequest};
use actor::Actor;
use actor::{add_stock, remove_stock, status};

pub fn start_random_actor(actor_id: usize, existing_markets: HashMap<usize, Sender<MarketMessages>>, actor_tx: Sender<ActorMessages>, actor_rx: Receiver<ActorMessages>) {
  let mut stop_flag = false;
  let mut init_history = false;
  let mut rng = rand::thread_rng();
  println!("Starting Random Actor {}", actor_id);
  let mut actor = Actor { id: actor_id,
                          money: 100,
                          stocks: HashMap::new(),
                          pending_money: 0,
                          pending_stock: (0, 0),
                          markets: existing_markets,
                          history: Arc::new(Mutex::new(MarketHistory {history: HashMap::new(), stocks: vec![]}))};

  for (_, market_tx) in actor.markets.iter() {
    market_tx.send(RegisterActor(actor.id, actor_tx.clone())).unwrap();
  }

  loop {
    if stop_flag {
      timer::sleep(Duration::milliseconds(1000));
      continue;
    }
    //Logic

    if init_history {
      for stock in actor.history.lock().unwrap().stocks.iter() {
          for (_, market_tx) in actor.markets.iter() {
            if actor.money != 0 {
              let p = rng.gen::<usize>() % actor.money + 1;
              let q = rng.gen::<usize>() % 150;
              let t = TransactionRequest {transaction_id: actor.id, actor_id: actor.id, stock_id: *stock, price: p, quantity: q};

              market_tx.send(BuyRequest(t)).unwrap();
            }
          }
      }
      for (stock, count) in actor.stocks.iter() {
        for (_, market_tx) in actor.markets.iter() {
          if *count != 0  {
            let p = rng.gen::<usize>() % 150 + 1;
            let q = rng.gen::<usize>() % *count + 1;
            let t = TransactionRequest {transaction_id: actor.id, actor_id: actor.id, stock_id: *stock, price: p, quantity: q};

            if q != 0 {
              market_tx.send(SellRequest(t)).unwrap();
            }
          }
        }
      }
    }

    let mark_clone = actor.markets.clone();
    let stock_clone = actor.stocks.clone();

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
                if actor.money >= money_request.amount && actor.money > 0 {
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
              init_history = true;},
            Time(_, _) => {},
            ReceiveActivityCount(_,_,_) => {},
            Stop(main_channel) => {
              main_channel.send((actor.id, "(Random Actor) ".to_string() + status(&actor).as_slice())).unwrap();
              stop_flag = true;
            }
          }
        },
      Err(TryRecvError::Empty) => {timer::sleep(Duration::milliseconds(1));},
      Err(TryRecvError::Disconnected) => {println!("ERROR: Actor {} disconnected", actor.id);}
    }
  }
}

fn has_pending_transaction(actor: &Actor) -> bool {
  (*actor).pending_money > 0 || (*actor).pending_stock.1 > 0
}
