use arrow::array::{
    ArrayBuilder, Int8Array, Int8Builder, Int64Array, Int64Builder, UInt32Array, UInt32Builder,
    UInt64Array, UInt64Builder,
};
use arrow::datatypes::SchemaRef;
use arrow::datatypes::{DataType, Field, Schema};
use arrow::record_batch::RecordBatch;
use common::{AggressorSide, Side};
use itch_parser::ItchMessage;
use itch_parser::parse_itch_message;
use orderbook::L3OrderBook;
use parquet::arrow::arrow_writer::ArrowWriter;
use std::fs::File;
use std::sync::Arc;

pub struct SymbolProcessor {
    pub symbol: String,
    pub stock_locate: u16,
    pub order_book: L3OrderBook,
    pub schema: SchemaRef,
    pub writer: Option<ArrowWriter<File>>,
    pub timestamp_builder: Int64Builder,
    pub msg_type_builder: Int8Builder,
    pub side_builder: Int8Builder,
    pub price_builder: UInt64Builder,
    pub qty_builder: UInt32Builder,
    pub ahead_builder: UInt32Builder,
    pub bid_builder: UInt64Builder,
    pub ask_builder: UInt64Builder,
    pub counter: u32, //use to check the number of rows matches what is in arrow database.
}

impl SymbolProcessor {
    pub fn flush_to_storage(&mut self) {
        let batch = RecordBatch::try_new(
            self.schema.clone(),
            vec![
                Arc::new(Int64Array::from(self.timestamp_builder.finish())),
                Arc::new(Int8Array::from(self.msg_type_builder.finish())),
                Arc::new(Int8Array::from(self.side_builder.finish())),
                Arc::new(UInt64Array::from(self.price_builder.finish())),
                Arc::new(UInt32Array::from(self.qty_builder.finish())),
                Arc::new(UInt32Array::from(self.ahead_builder.finish())),
                Arc::new(UInt64Array::from(self.bid_builder.finish())),
                Arc::new(UInt64Array::from(self.ask_builder.finish())),
            ],
        )
        .unwrap(); //fix : dont use unwraps....

        self.writer
            .as_mut()
            .expect("Writer should be initialized")
            .write(&batch)
            .expect("Writing to Parquet failed");
    }

    pub fn record_snapshot(
        &mut self,
        ts: u64,
        msg_type: char,
        side: i8,
        price: u64,
        qty: u32,
        ahead: u32,
    ) {
        self.timestamp_builder.append_value(ts as i64);
        self.msg_type_builder.append_value(msg_type as i8);
        self.side_builder.append_value(side);
        self.price_builder.append_value(price);
        self.qty_builder.append_value(qty);
        self.ahead_builder.append_value(ahead);

        let (bid, _) = self.order_book.get_best_bid().unwrap_or((0, 0));
        let (ask, _) = self.order_book.get_best_ask().unwrap_or((0, 0));
        self.bid_builder.append_value(bid);
        self.ask_builder.append_value(ask);

        if self.timestamp_builder.len() >= 1024 {
            self.flush_to_storage();
        }
    }

    pub fn handle_event(&mut self, msg: ItchMessage) {
        match msg {
            ItchMessage::AddOrder {
                stock_locate: _,
                order_id,
                side,
                quantity,
                price,
                timestamp,
            } => {
                self.order_book.add_order(order_id, side, price, quantity);

                let ahead = self
                    .order_book
                    .orders
                    .get(&order_id)
                    .map(|o| o.shares_ahead)
                    .unwrap_or(0);
                self.record_snapshot(timestamp, 'A', side as i8, price, quantity, ahead);
                self.counter += 1;
            }

            ItchMessage::ModifyOrder {
                stock_locate: _,
                order_id,
                quantity_to_remove,
                timestamp,
            } => {
                let mut ahead = 0;
                let mut price = 0;
                let mut side = 0;

                if let Some(order) = self.order_book.orders.get(&order_id) {
                    ahead = order.shares_ahead;
                    price = order.price;
                    side = order.side as i8;
                }

                self.order_book.cancel_order(order_id, quantity_to_remove);
                self.record_snapshot(timestamp, 'X', side, price, quantity_to_remove, ahead);
                self.counter += 1;
            }

            ItchMessage::DeleteOrder {
                stock_locate: _,
                order_id,
                timestamp,
            } => {
                if let Some(order) = self.order_book.orders.get(&order_id) {
                    let current_qty = order.quantity;

                    let ahead = order.shares_ahead;
                    let price = order.price;
                    let side = order.side as i8;

                    self.order_book.cancel_order(order_id, current_qty);

                    self.record_snapshot(timestamp, 'D', side, price, current_qty, ahead);
                    self.counter += 1;
                }
            }
            //Execution: Someone else bought those shares. The shares must leave the book.
            ItchMessage::OrderExecuted {
                stock_locate: _,
                order_id,
                quantity,
                match_number: _,
                timestamp,
            } => {
                let trade_info = if let Some(order) = self.order_book.orders.get(&order_id) {
                    let side = match order.side {
                        Side::Buy => AggressorSide::Seller,
                        Side::Sell => AggressorSide::Buyer,
                    };
                    Some((order.price, side, order.shares_ahead))
                } else {
                    None
                };

                if let Some((price, side, shares_ahead)) = trade_info {
                    self.order_book.cancel_order(order_id, quantity);
                    self.record_snapshot(timestamp, 'E', side as i8, price, quantity, shares_ahead);
                    self.counter += 1;
                }
            }

            // Remove liquidity at original price, but record trade at 'price'
            ItchMessage::OrderExecutedWithPrice {
                stock_locate: _,
                order_id,
                quantity,
                price: _,
                match_number: _,
                timestamp,
            } => {
                if let Some(order) = self.order_book.orders.get(&order_id) {
                    //If the order in the book was a Bid, the person who executed against it is a seller and vice versa.
                    let side = match order.side {
                        Side::Buy => AggressorSide::Seller,
                        Side::Sell => AggressorSide::Buyer,
                    };

                    let ahead = order.shares_ahead;
                    let price = order.price;

                    self.order_book.cancel_order(order_id, quantity);
                    self.record_snapshot(timestamp, 'C', side as i8, price, quantity, ahead);
                    self.counter += 1;
                }
            }

            ItchMessage::OrderReplaced {
                stock_locate: _,
                old_order_id,
                new_order_id,
                quantity,
                price,
                timestamp,
            } => {
                if let Some(orig_order) = self.order_book.orders.get(&old_order_id) {
                    let side = orig_order.side;
                    let old_total_qty = orig_order.quantity;

                    //Kill the old order
                    self.order_book.cancel_order(old_order_id, old_total_qty);

                    //Add the new order
                    self.order_book
                        .add_order(new_order_id, side, price, quantity);

                    let new_ahead = self
                        .order_book
                        .orders
                        .get(&new_order_id)
                        .map(|o| o.shares_ahead)
                        .unwrap_or(0);

                    self.record_snapshot(timestamp, 'U', side as i8, price, quantity, new_ahead);

                    self.counter += 1;
                }
            }

            _ => {}
        }
    }
}

pub struct StockDataHandler {
    pub processors: Vec<SymbolProcessor>,
    pub lookup_table: [i16; 65536],
    pub counter_dropped: u64,
    pub counter_symbol_processed: u64,
}

impl StockDataHandler {
    pub fn create_schema() -> SchemaRef {
        Arc::new(Schema::new(vec![
            Field::new("timestamp", DataType::Int64, false),
            Field::new("msg_type", DataType::Int8, false),
            Field::new("side", DataType::Int8, false),
            Field::new("price", DataType::UInt64, false),
            Field::new("quantity", DataType::UInt32, false),
            Field::new("shares_ahead", DataType::UInt32, false),
            Field::new("bid_price", DataType::UInt64, false),
            Field::new("ask_price", DataType::UInt64, false),
        ]))
    }

    pub fn new(target_symbols: Vec<String>) -> Self {
        let mut processors = Vec::new();

        for sym in target_symbols {
            let file = File::create(format!("{}_l3.parquet", sym)).unwrap();
            let schema = StockDataHandler::create_schema();
            let writer = ArrowWriter::try_new(file, schema.clone(), None).unwrap();

            processors.push(SymbolProcessor {
                symbol: sym,
                stock_locate: 0,
                order_book: L3OrderBook::new(),
                schema,
                writer: Some(writer),
                timestamp_builder: Int64Builder::with_capacity(1024),
                msg_type_builder: Int8Builder::with_capacity(1024),
                side_builder: Int8Builder::with_capacity(1024),
                price_builder: UInt64Builder::with_capacity(1024),
                qty_builder: UInt32Builder::with_capacity(1024),
                ahead_builder: UInt32Builder::with_capacity(1024),
                bid_builder: UInt64Builder::with_capacity(1024),
                ask_builder: UInt64Builder::with_capacity(1024),
                counter: 0,
            });
        }

        Self {
            processors,
            lookup_table: [-1; 65536],
            counter_dropped: 0,
            counter_symbol_processed: 0,
        }
    }

    pub fn handle_stock_directory(&mut self, stock_locate: u16, symbol: String) {
        let clean_symbol = symbol.trim_matches(char::from(0)).trim();

        if let Some(pos) = self
            .processors
            .iter()
            .position(|p| p.symbol == clean_symbol)
        {
            self.processors[pos].stock_locate = stock_locate;
            self.lookup_table[stock_locate as usize] = pos as i16;
            println!(
                "Successfully mapped {} to stock locate ID: {}",
                clean_symbol, stock_locate
            );
        }
    }

    pub fn handle_raw_packet(&mut self, raw_data: &[u8]) {
        let locate = u16::from_be_bytes([raw_data[1], raw_data[2]]);
        let slot_idx = self.lookup_table[locate as usize];

        if slot_idx != -1 {
            if let Some(msg) = parse_itch_message(raw_data) {
                let processor = &mut self.processors[slot_idx as usize];
                processor.handle_event(msg);
                self.counter_symbol_processed += 1;
            }
        } else {
            let msg_type = raw_data[0];
            if msg_type == b'R' {
                if let Some(msg) = parse_itch_message(raw_data) {
                    self.filter_message_for_symbols(msg);
                }
            } else {
                self.counter_dropped += 1;
            }
        }
    }

    fn filter_message_for_symbols(&mut self, msg: ItchMessage) {
        match msg {
            ItchMessage::StockDirectory {
                stock_locate,
                symbol,
            } => {
                self.handle_stock_directory(stock_locate, symbol);
            }
            _ => {}
        }
    }

    pub fn finalize(&mut self) {
        println!();
        for processor in &mut self.processors {
            if !processor.timestamp_builder.is_empty() {
                processor.flush_to_storage();
            }

            if let Some(wrt) = processor.writer.take() {
                wrt.close().expect("Failed to close parquet writer");
                println!(
                    "Finalized {}. {} orderbook snapshots taken.",
                    processor.symbol, processor.counter
                );
            }
        }
    }
}
