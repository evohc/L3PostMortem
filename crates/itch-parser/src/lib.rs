use common::Side;

pub enum ItchMessage {
    //Message type 'A' : new order
    AddOrder {
        stock_locate: u16,
        order_id: u64,
        side: Side,
        quantity: u32,
        price: u64,
        timestamp: u64,
    },
    //Message type 'E' or 'X' : Existing order is reduced/removed
    ModifyOrder {
        stock_locate: u16,
        order_id: u64,
        quantity_to_remove: u32,
        timestamp: u64,
    },
    //Message type 'D' delete order
    DeleteOrder {
        stock_locate: u16,
        order_id: u64,
        timestamp: u64,
    },
    //Message type 'E' order excuted
    OrderExecuted {
        stock_locate: u16,
        order_id: u64,
        quantity: u32,
        match_number: u64,
        timestamp: u64,
    },
    //Message type 'C' order excuted with price
    OrderExecutedWithPrice {
        stock_locate: u16,
        order_id: u64,
        quantity: u32,
        price: u64,
        match_number: u64,
        timestamp: u64,
    },

    //Message type 'U' replace order
    OrderReplaced {
        stock_locate: u16,
        old_order_id: u64,
        new_order_id: u64,
        quantity: u32,
        price: u64,
        timestamp: u64,
    },

    //Message Type 'R' stock directory listing for day
    StockDirectory {
        stock_locate: u16,
        symbol: String,
    },
}

fn parse_timestamp(data: &[u8]) -> u64 {
    // Grab the first 2 bytes of the timestamp (Bytes 5 and 6)
    // Offset 5..7 is the high 16 bits
    let high = u16::from_be_bytes([data[5], data[6]]) as u64;

    // Grab the next 4 bytes (Bytes 7, 8, 9, 10)
    // Offset 7..11 is the low 32 bits
    let low = u32::from_be_bytes([data[7], data[8], data[9], data[10]]) as u64;

    // Bring them together
    // Move the 'high' bytes 32 bits to the left to make room for 'low'
    (high << 32) | low
}

pub fn parse_itch_message(data: &[u8]) -> Option<ItchMessage> {
    //first byte is message type
    let message_type = data[0] as char;
    let stock_locate = u16::from_be_bytes(data[1..3].try_into().ok()?); //same for all messages

    match message_type {
        /*
        STOCK DIRECTORY:
            Field Name	          Offset  Len
            Stock Directory Msg   0	      1
            Stock Locate	      1	      2   The unique ID assigned for the day.
            Tracking Num	      3	      2
            Timestamp	          5	      6
            Stock       	      11	  8   The actual symbol (e.g., "AAPL    ")
            ....
        */
        'R' => {
            let symbol = String::from_utf8_lossy(&data[11..19]).trim().to_string(); //heap allocation here this is pre trading but fix it.

            Some(ItchMessage::StockDirectory {
                stock_locate,
                symbol,
            })
        }

        /*
        ADD ORDER:
            Field Name	    Offset	Len
            Message Type	0	    1
            Stock Locate	1	    2
            Tracking Num	3	    2
            Timestamp	    5	    6
            Order Ref Num	11	    8
            Buy/Sell 	    19	    1
            Shares	        20	    4
            Stock	        24	    8
            Price	        32	    4
        */
        'A' => {
            let order_id = u64::from_be_bytes(data[11..19].try_into().unwrap());

            let side = match data[19] as char {
                'B' => Side::Buy,
                'S' => Side::Sell,
                _ => return None,
            };

            let quantity = u32::from_be_bytes(data[20..24].try_into().unwrap());
            let price = u32::from_be_bytes(data[32..36].try_into().unwrap()) as u64;
            let timestamp = parse_timestamp(data);

            Some(ItchMessage::AddOrder {
                stock_locate,
                order_id,
                side,
                quantity,
                price,
                timestamp,
            })
        }

        /*
        CANCEL ORDER (partial)
            Field Name 	    Offset	 Len
            Message Type	0	     1
            Stock Locate	1	     2
            Tracking Num	3	     2
            Timestamp	    5	     6
            Order Ref Num	11	     8
            Canceled Shares	19	     4
        */
        'X' => {
            let order_id = u64::from_be_bytes(data[11..19].try_into().unwrap());
            let quantity_to_remove = u32::from_be_bytes(data[19..23].try_into().unwrap());
            let timestamp = parse_timestamp(data);

            Some(ItchMessage::ModifyOrder {
                stock_locate,
                order_id,
                quantity_to_remove,
                timestamp,
            })
        }

        /*
        DELETE ORDER
            Field Name 	    Offset	Len
            Message Type	0	    1
            Stock Locate	1	    2
            Tracking Num	3	    2
            Timestamp	    5	    6
            Order Ref Num	11	    8
         */
        'D' => {
            let order_id = u64::from_be_bytes(data[11..19].try_into().unwrap());
            let timestamp = parse_timestamp(data);

            Some(ItchMessage::DeleteOrder {
                stock_locate,
                order_id,
                timestamp,
            })
        }
        /*
        ORDER EXECUTED
            Field Name	   Offset	Len
            Message Type	0	    1
            Stock Locate	1	    2
            Tracking Num	3	    2
            Timestamp	    5	    6
            Order Ref Num	11	    8
            Executed Shares	19	    4
            Match Number	23	    8
        */
        'E' => {
            let order_id = u64::from_be_bytes(data[11..19].try_into().unwrap());
            let quantity = u32::from_be_bytes(data[19..23].try_into().unwrap());
            let match_number = u64::from_be_bytes(data[23..31].try_into().unwrap());
            let timestamp = parse_timestamp(data);

            Some(ItchMessage::OrderExecuted {
                stock_locate,
                order_id,
                quantity,
                match_number,
                timestamp,
            })
        }

        /*
        ORDER EXECUTED PRICE
            Field Name	         Offset	Len
            Message Type	     0	    1
            Stock Locate	     1	    2
            Tracking Num	     3	    2
            Timestamp	         5	    6
            Order Ref Num	    11	    8
            Executed Shares	    19	    4
            Match Number	    23	    8
            Printable           31      1
            Execution Price     32      4
        */
        'C' => {
            let order_id = u64::from_be_bytes(data[11..19].try_into().unwrap());
            let quantity = u32::from_be_bytes(data[19..23].try_into().unwrap());
            let match_number = u64::from_be_bytes(data[23..31].try_into().unwrap());
            let price: u64 = u32::from_be_bytes(data[32..36].try_into().unwrap()) as u64;
            let timestamp = parse_timestamp(data);

            Some(ItchMessage::OrderExecutedWithPrice {
                stock_locate,
                order_id,
                quantity,
                price,
                match_number,
                timestamp,
            })
        }

        /*
        Replace Order
            Name                Offset Len
            Message Type        0      1
            Stock Locate        1      2
            Tracking Number     3      2
            Timestamp           5      6
            Original Order      11     8
            New Order           19     8
            Shares              27     4
            Price               31     4
        */
        'U' => {
            let old_order_id = u64::from_be_bytes(data[11..19].try_into().unwrap());
            let new_order_id = u64::from_be_bytes(data[19..27].try_into().unwrap());
            let quantity = u32::from_be_bytes(data[27..31].try_into().unwrap());
            let price: u64 = u32::from_be_bytes(data[31..35].try_into().unwrap()) as u64;
            let timestamp = parse_timestamp(data);

            Some(ItchMessage::OrderReplaced {
                stock_locate,
                old_order_id,
                new_order_id,
                quantity,
                price,
                timestamp,
            })
        }

        _ => None,
    }
}
