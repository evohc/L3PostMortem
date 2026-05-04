import polars as pl
import pandas as pd
import matplotlib.pyplot as plt

##Display market stats.
## By knowning the gap between joining a queue and being filled, one can calculate the fill probability of an order.
def display_market_state(file_path):
    print(f"Processing {file_path}.")
    df = pl.read_parquet(file_path)

    msg_map = {65: 'Add', 68: 'Delete', 69: 'Executed', 67: 'ExecutedWithPrice', 85: 'Replace', 88: 'Modify'}

    queue_stats = df.group_by("msg_type").agg([
    pl.col("shares_ahead").mean().alias("Avg Ahead"),
    pl.col("shares_ahead").max().alias("Max Ahead"),
    pl.len().alias("Count")
    ])

    queue_stats = queue_stats.with_columns(
    pl.col("msg_type").cast(pl.Utf8).replace(msg_map).alias("Order Type")
    ).select([
        "Order Type",
        "Avg Ahead", 
        "Max Ahead", 
        "Count"
    ])

    print(queue_stats)

display_market_state("../target/release/AAPL_l3.parquet")
display_market_state("../target/release/MSFT_l3.parquet")