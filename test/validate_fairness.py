import polars as pl
import pandas as pd
import matplotlib.pyplot as plt

## Validates the integrity of the engine's shares ahead logic.
## Since NASDAQ uses a FIFO model, queue position must depend only on arrival time, not order size.
## A correlation near 0 between quantity and shares_ahead proves the engine 
## maintains "FIFO fairness," treating 10-share and 10,000-share orders identically.


def validate_fairness(file_path):
    print(f"Processing {file_path}.")
    df = pl.read_parquet(file_path)

    long_queues = df.filter(pl.col("shares_ahead") > 1000)

    if not long_queues.is_empty():
        print(f"Found {len(long_queues)} events with over 1000 shares ahead.")
        correlation = long_queues.select(
        pl.corr("quantity", "shares_ahead").alias("Quantity Queue Correlation")
        )
        print(correlation)
    else:
        print("No long queues found in this sample.")


validate_fairness("../target/release/AAPL_l3.parquet")
validate_fairness("../target/release/MSFT_l3.parquet")
