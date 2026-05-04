import polars as pl
import pandas as pd
import matplotlib.pyplot as plt

def validate_snapshot(file_path):
    print(f"Validating {file_path}.")
    df = pl.read_parquet(file_path)

    ##Check if a snapshot is out of order.
    ## diff() subtracts the previous row's timestamp from the current row's, if -ive, it means a snapshot was processed out of order.
    out_of_order_snapshots = (df['timestamp'].diff() < 0).sum()
    
    ##Check if highest bid is offering more than the lowest seller.
    ## The exchange matching engine would have already executed those orders against each other
    crossed_book = df.filter((pl.col("bid_price") >= pl.col("ask_price")) & (pl.col("ask_price") > 0))

    ##Shares ahead should never be negative.
    negative_share_ahead = df.select(pl.col("shares_ahead") < 0).sum().item()

    print(f"Total Rows: {len(df)}")
    print(f"Out of order snapshots: {out_of_order_snapshots}")
    print(f"Crossed book instances: {len(crossed_book)}")
    print(f"Negative queue errors: {negative_share_ahead}")
    
    if out_of_order_snapshots == 0 and len(crossed_book) == 0 and negative_share_ahead == 0:
        print("Data verified.")

validate_snapshot("../target/release/AAPL_l3.parquet")
validate_snapshot("../target/release/MSFT_l3.parquet")