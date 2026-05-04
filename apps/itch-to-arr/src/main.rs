use memmap2::MmapOptions;
use std::fs::File;
use std::time::Instant;
use stock_handler::StockDataHandler;

//We need to add" This is a great "Validation" metric to show the user: "Processed 100M messages, tracked 5 symbols (2M events), skipped 98M events."

fn main() {
    println!("Starting replay engine.");

    let core_ids = core_affinity::get_core_ids().expect("Failed to read CPU cores.");
    if core_ids.len() > 2 {
        core_affinity::set_for_current(core_ids[2]);
        println!("Replay engine pinned to Core 2.");
    } else {
        println!("Cant grab a core, continue...");
    }

    // well known high volatility day for testing...01302020.NASDAQ_ITCH50
    // calm day 10302019.NASDAQ_ITCH50.
    let data_file = File::open("/home/amurray/Downloads/10302019.NASDAQ_ITCH50")
        .expect("Cant open 10302019.NASDAQ_ITCH50...");
    let mmap = unsafe { MmapOptions::new().map(&data_file) }.expect("Read only memory map failed.");

    let symbols = vec!["AAPL".to_string(), "MSFT".to_string()];

    let mut stock_handler = StockDataHandler::new(symbols.clone());

    let mut cursor = 0;
    let mut num_of_packets_sent: u64 = 0;

    let now = Instant::now();

    while (cursor + 2) < mmap.len() {
        let msg_len: usize = u16::from_be_bytes([mmap[cursor], mmap[cursor + 1]]) as usize;

        let msg_start = cursor + 2;
        let msg_end = msg_start + msg_len;

        if msg_end > mmap.len() {
            break;
        }

        let msg_payload = &mmap[msg_start..msg_end];

        stock_handler.handle_raw_packet(msg_payload);

        num_of_packets_sent += 1;
        cursor = msg_end;
    }

    stock_handler.finalize();

    let elapsed = now.elapsed();
    let exact_seconds = elapsed.as_secs_f64();
    let packets_per_second = (num_of_packets_sent as f64) / exact_seconds;

    println!(
        "\n{} ITCH packets processed in {} secs. Rate: {:.0} packets/sec.\n{} symbols processed.\n{} events processed, {} events dropped.",
        num_of_packets_sent,
        elapsed.as_secs(),
        packets_per_second,
        symbols.len(),
        stock_handler.counter_symbol_processed,
        stock_handler.counter_dropped,
    );
}
