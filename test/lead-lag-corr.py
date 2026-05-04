import polars as pl
import matplotlib.pyplot as plt
import numpy as np

## See if a pattern exists between the 2 data sets... i.e does a change in appl reflect a change in msft or vice versa

aapl = pl.read_parquet("../target/release/AAPL_l3.parquet").select(["timestamp", "bid_price", "ask_price"])
msft = pl.read_parquet("../target/release/MSFT_l3.parquet").select(["timestamp", "bid_price", "ask_price"])

##Get mid price for both.
def prep_midprice(df, name):
    return df.with_columns(
        ((pl.col("bid_price") + pl.col("ask_price")) / 2).alias(f"{name}_mid")
    ).select(["timestamp", f"{name}_mid"])

aapl = prep_midprice(aapl, "AAPL")
msft = prep_midprice(msft, "MSFT")

#Create a single table where every AAPL update is matched with the most recent MSFT price.
synced = aapl.join_asof(msft, on="timestamp", strategy="backward")

##Runs a loop from -50 to +50 to do a lag test.
##At Lag 0: compare AAPL's price now to MSFT's price now.
##At Lag 25: compare AAPL's price now to MSFT's price 25 events in the future.
lags = range(-50, 51)
correlations = []

for k in lags:
    corr = synced.select(
        pl.corr("AAPL_mid", pl.col("MSFT_mid").shift(k))
    ).to_series()[0]
    correlations.append(corr)

## Find where the correlation peaks 
best_lag_index = np.argmax(correlations)
best_lag = lags[best_lag_index]
max_corr = correlations[best_lag_index]

print(f"Optimal Lag: {best_lag} events")
print(f"Max Correlation: {max_corr:.4f}")

## Display data
plt.figure(figsize=(10, 5))
plt.plot(lags, correlations, marker='o', linestyle='-', color='blue')
plt.axvline(x=0, color='red', linestyle='--', label='Time 0') ## Lag 0 of both
plt.axvline(x=best_lag, color='green', linestyle='-', label=f'Best Lag ({best_lag})')

plt.title("AAPL vs MSFT lead lag profile")
plt.xlabel("Lag (Number of Events)")
plt.ylabel("Pearson Correlation")
plt.legend()
plt.grid(True, alpha=0.3)
plt.show()