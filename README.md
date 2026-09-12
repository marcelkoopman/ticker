# Ticker 🚀 (Beta)

[![Beta Status](https://img.shields.io/badge/Status-Beta-yellow)](<>)

A menubar price ticker for Bitcoin, Gold, and TTF Gas.

## Features

- 📊 Real-time price updates
- 🔄 Auto-refresh every 5 minutes
- 💾 Day-change tracking (open of the local day)
- 👁 **Price watches**: set a threshold at the current price; when the price rises above it you get a menubar alert icon and a macOS notification popup

### Price watch usage

1. Wait for prices to appear in the menubar.
2. Open the menu → choose **📌 Set watch: &lt;asset&gt; @ &lt;price&gt;** for the asset you care about.
3. When the live price goes **above** that threshold:
   - the tray icon switches to the alert state
   - a macOS notification is shown once per breach
4. Clear a watch with **🗑 Clear watch: &lt;asset&gt;**.

Watches are stored in `~/.ticker_price_history.json`.

## Installation

Download from [Releases](https://github.com/marcelkoopman/ticker/releases)

## Requirements

- macOS 12+

## License

MIT
