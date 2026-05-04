use common::Side;
use std::collections::{BTreeMap, HashMap};

/*
What I missed in the first OB

Imagine the book at Price $150 looks like this:
    Order 101: 50 shares (shares_ahead: 0)
    Order 105: 30 shares (shares_ahead: 50)
    Order 110: 20 shares (shares_ahead: 80) —> ME.

Scenario A: Order 101 is Cancelled (50 shares)
    Your ID (110) is greater than the cancelled ID (101).
    That means 101 was in front of you.
    We must "slide" you forward: Your shares_ahead becomes $80 - 50 = 30$.

Scenario B: A new Order 120 arrives (100 shares)
    Your ID (110) is smaller than the new ID (120).
    That means 120 is behind you.
    Your shares_ahead does not change. You are still in the same spot.

Add this metric to the runtime and also make it available for backtesting.

In research the trader looks at the arrow file to find the "patterns. e.g.
 "Every time the 'shares_ahead' for AAPL drops below 500, the price jumps 1 cent within 50ms."

 At runtime...
 "I just saw a 'Cancel' message. My internal L3OrderBook says shares_ahead is now 450. 
 Research says 450 is the 'Magic Number' to buy more. Action: Buy.

*/

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Order {
    pub price: u64,
    pub quantity: u32,
    pub side: Side,
    pub shares_ahead: u32,
}

pub struct L3OrderBook {
    pub orders: HashMap<u64, Order>,
    //Key:Price, Value: Total Quantity at that price.
    pub bids: BTreeMap<u64, u32>, //Best Bid: This is the last (highest) price.
    pub asks: BTreeMap<u64, u32>, //Best Ask: It's the first (lowest) price.
}

impl L3OrderBook {
    pub fn new() -> Self {
        Self {
            orders: HashMap::new(),
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
        }
    }

    pub fn add_order(&mut self, id: u64, side: Side, price: u64, quantity: u32) {
        let book_side = match side {
            Side::Buy => &mut self.bids,
            Side::Sell => &mut self.asks,
        };

        let shares_ahead = book_side.get(&price).copied().unwrap_or(0);

        let new_order = Order {
            price,
            quantity,
            side,
            shares_ahead,
        };

        self.orders.insert(id, new_order);

        *book_side.entry(price).or_insert(0) += quantity;
    }

    /*Find order num in the HashMap to get price and type.
    Subtract the cancelled shares from the specific order.
    In BTreeMap for price, subtract the shares from the public total.
    If the specific Order hits 0 shares, delete from HashMap.
    If the price level hits 0 shares, delete fromBTreeMap.*/

    pub fn cancel_order(&mut self, id: u64, quantity_to_cancel: u32) {
        let (price, side, actual_order_qty) = match self.orders.get(&id) {
            Some(o) => (o.price, o.side, o.quantity),
            None => return,
        };

        // Ensure we don't cancel more than the order actually has
        let effective_cancel = std::cmp::min(quantity_to_cancel, actual_order_qty);

        // 1. Update individual order
        if let Some(order) = self.orders.get_mut(&id) {
            order.quantity -= effective_cancel;
            if order.quantity == 0 {
                self.orders.remove(&id);
            }
        }

        // 2. Update price level total
        let book_side = match side {
            Side::Buy => &mut self.bids,
            Side::Sell => &mut self.asks,
        };

        if let Some(total_qty) = book_side.get_mut(&price) {
            // Safety check to prevent wrap-around
            if effective_cancel >= *total_qty {
                book_side.remove(&price);
            } else {
                *total_qty -= effective_cancel;
            }
        }
    }
    // Returns (Price, Total Quantity) for the best buyer
    pub fn get_best_bid(&self) -> Option<(u64, u32)> {
        //.next_back() gets the highest price level, iterator gives you references to the data inside the map,controls memory
        // and returns Option<(&u64, &u32)>, .map() ->transform value into something else...// Extract the values from the pointers
        // return them as a pair
        //self.bids.iter().next_back().map(|(&p, &q)| (p, q))

        //this is easier
        let mut iterator = self.bids.iter();

        let best_bid_ref = iterator.next_back();

        let result = best_bid_ref.map(|(&p, &q)| {
            let pair = (p, q);
            return pair;
        });

        return result;
    }

    // Returns (Price, Total Quantity) for the best seller
    pub fn get_best_ask(&self) -> Option<(u64, u32)> {
        self.asks.iter().next().map(|(&p, &q)| (p, q))
    }

    pub fn spread(&self) -> Option<u64> {
        if let (Some((bid, _)), Some((ask, _))) = (self.get_best_bid(), self.get_best_ask()) {
            return Some(ask.saturating_sub(bid));
        }
        None
    }
}
