# HackTrader - Market Making Simulator

A market making game running on the 16-bit Hack computer platform.
You act as a market maker: quote bid/ask prices on an option, manage inventory risk,
and maximize your P&L over 30 rounds.

## Features

- Live order book with price-time priority matching
- Black-Scholes options pricing in fixed-point integer arithmetic
- Greeks (Delta, Gamma, Theta, Vega) via lookup tables and interpolation
- Random walk price engine with configurable volatility
- Signed trade confirmations (hash-based signatures)
- Three difficulty levels
- P&L tracking with equity curve visualization
- Multi-density dithered graphics (25%/50%/75% perceived grayscale)
- Graphical depth bars, gradient fills, and block-letter rendering

## Architecture

9 Jack classes, ~2700 lines:

| File | Purpose |
|------|---------|
| Main.jack | Entry point, splash screen, dithering, block letters |
| HackTrader.jack | Game controller, round loop, scoring |
| OrderBook.jack | Matching engine, price levels, depth |
| MarketMaker.jack | Player state: position, P&L, quotes |
| OptionsPricer.jack | Black-Scholes pricing, Greeks |
| PriceEngine.jack | Underlying price random walk |
| TradeHasher.jack | Trade hash signatures (crypto touch) |
| UI.jack | Screen rendering, depth bars, equity curve, trade log |
| RNG.jack | PRNG for order flow and price moves |

## Build

Requires the nand2tetris tools (JackCompiler + VMEmulator) with Java installed.

### Clean

```bash
rm -f projects/09/HackTrader/*.vm
```

### Compile

From the repository root:

```bash
tools/JackCompiler.sh projects/09/HackTrader/
```

This produces one `.vm` file per `.jack` source file.

### Run

1. Open the **VM Emulator** (`tools/VMEmulator.sh`)
2. File > Load Program > select the `projects/09/HackTrader/` directory
3. Set speed to "Fast" or "No animation" for playable frame rate
4. Click Run

## How to Play

### Splash Screen

Select difficulty:
- **1** - Easy (low volatility, slow order flow)
- **2** - Medium (mid volatility, mid flow)
- **3** - Hard (high volatility, fast flow)
- **Q** - Quit

Press any key after selection to seed the RNG and start.

### Controls

| Key | Action |
|-----|--------|
| B | Buy (market order) |
| S | Sell (market order) |
| W | Widen your bid/ask spread |
| N | Narrow your bid/ask spread |
| + | Increase quote size |
| - | Decrease quote size |
| Q | Quit game early |

### Screen Layout

```
▓▓▓▓ HACKTRADER - Market Making Sim ▓▓▓▓
─────────────────────────────────────────
 ASK                     ║ ▓OPTIONS▓▓▓▓▓
 102.0: ▒▒▒▒▒           ║  Call: 0.3
 101.3: ▒▒              ║  K:100.0  S:100.0
 101.0: ▒▒▒             ║ ‒ ‒ ‒ ‒ ‒ ‒ ‒ ‒
 100.6: ▒               ║  Delta: 0
░░spread: 1.6░░░░░░░░░░░║  Gamma: 0
  99.0: ██               ║  Theta: 0
  98.0: █████            ║  Vega:  0
  97.9: ███              ║
  97.0: ████████         ║
Last:101.0  Vol:2        ║
─────────────────────────────────────────
▓▓YOUR POSITION▓▓▓▓▓▓▓▓║▓▓TRADE LOG▓▓▓▓
Pos:+6 Avg:99.0         ║ BUY  2099.0 39DB
uPnL:+0.6 rPnL:0.0     ║ BUY  2099.0 385C
Bid:98.8 Ask:100.8 Sz:2 ║ BUY  2099.0 1FD1
░TOTAL PnL: +0.6░░░░░░░░║
░[B]uy [S]ell [W]iden [N]arrow░░░░░░░░░░
░[+]Size [-]Size [Q]uit   Rd:19/30░░░░░░
·········································
 Game started! Manage your quotes.
▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄
```

Key: `█` = solid bar (bids), `▒` = hollow bar (asks),
`▓` = 50% dither header, `░` = 25% dither highlight,
`║` = double-line divider, `‒` = dashed separator, `·` = dotted separator

### How It Works (No Finance Background Needed)

Think of yourself as a **shopkeeper** who buys and sells one item — like a currency
exchange booth at an airport. The item's value keeps changing, and your job is to
profit from the gap between your buying and selling prices without getting stuck
holding too much when the price moves against you.

Here's what each part of the screen means:

```
▓▓▓▓ HACKTRADER - Market Making Sim ▓▓▓▓        ← Title bar
─────────────────────────────────────────
 ASK                     ║ ▓OPTIONS▓▓▓▓▓        ← Right panel: price stats
 102.0: ▒▒▒▒▒           ║  Call: 0.3               (ignore if unfamiliar —
 101.3: ▒▒              ║  K:100.0  S:100.0         it's just extra info
 101.0: ▒▒▒             ║ ‒ ‒ ‒ ‒ ‒ ‒ ‒ ‒          for advanced players)
 100.6: ▒               ║  Delta: 0
░░spread: 1.6░░░░░░░░░░░║  Gamma: 0
  99.0: ██               ║  Theta: 0
  98.0: █████            ║  Vega:  0
  97.9: ███              ║
  97.0: ████████         ║
Last:101.0  Vol:2        ║
─────────────────────────────────────────
▓▓YOUR POSITION▓▓▓▓▓▓▓▓║▓▓TRADE LOG▓▓▓▓
Pos:+6 Avg:99.0         ║ BUY  2@99.0 39DB
uPnL:+0.6 rPnL:0.0     ║ BUY  2@99.0 385C
Bid:98.8 Ask:100.8 Sz:2 ║ BUY  2@99.0 1FD1
░TOTAL PnL: +0.6░░░░░░░░║
```

**The order book (center-left)** — your "shop window":
- Lines above `spread` with `▒` bars = prices others are willing to **sell** at
  (the cheapest is closest to the middle — that's the best deal for you to buy)
- Lines below `spread` with `█` bars = prices others are willing to **buy** at
  (the highest is closest to the middle — that's the best deal for you to sell)
- The `spread: 1.6` line = the gap between the cheapest sell and the highest buy.
  This gap is where you make money: buy at the lower price, sell at the higher one.

**Your position (bottom-left)** — your inventory and profit:
- `Pos:+6` = you currently hold 6 units (bought more than you sold)
- `Avg:99.0` = you paid 99.0 on average for them
- `uPnL:+0.6` = if you sold now, you'd make 0.6 (unrealized profit)
- `rPnL:0.0` = profit already locked in from completed round-trips
- `Bid:98.8 Ask:100.8 Sz:2` = your current quotes: willing to buy at 98.8,
  sell at 100.8, 2 units at a time
- `TOTAL PnL` = your score (realized + unrealized)

**Trade log (bottom-right)** — recent fills:
- Each line shows a trade that happened (e.g., `BUY 2@99.0 39DB`)
- The 4-character hex code (`39DB`) is a unique trade ID

**Controls in plain terms:**

| Key | What it does |
|-----|-------------|
| W / N | Make your profit margin wider (safer, fewer trades) or narrower (riskier, more trades) |
| + / - | Offer to trade bigger or smaller quantities at a time |
| B / S | Manually buy or sell to reduce your inventory if it's gotten too large |
| Q | Quit |

**The rhythm of play:**
1. You set your buy/sell prices and the quantity you're offering
2. Computer-controlled traders randomly buy from you or sell to you
3. Each trade changes your inventory (you might end up holding +5 or -3 units)
4. The price ticks up or down randomly each round
5. After 30 rounds, your score = total profit from all trades + value of remaining inventory

**One sentence version:** *You're setting buy/sell prices on a product whose value
keeps changing — wide margins are safe but slow, tight margins earn more but risk
losses, and holding too much inventory when the price drops wipes you out.*

### Game Rules (Detailed)

1. **You are a market maker.** You continuously quote a bid (buy) and ask (sell) price.
   Bot order flow arrives and may fill your quotes.

2. **Each round (~200ms):**
   - The underlying price moves (random walk)
   - Options Greeks are recalculated
   - Bots submit market/limit orders
   - Your quotes may get filled
   - The order book is replenished if thin

3. **Managing risk:**
   - Widen spread (W) to reduce fill probability but earn more per fill
   - Narrow spread (N) to increase fills but with less edge
   - Buy/Sell manually (B/S) to flatten your position
   - Watch Delta to understand your directional exposure

4. **The trade signature** (4-hex-char code after each fill, e.g. `7A3F`) is a hash of trade details — a nod to cryptographic trade verification.

5. **Game ends** after 30 rounds. Your total P&L (realized + unrealized) determines your rating.

### Scoring

| P&L | Rating |
|-----|--------|
| < 0 | Blown up! You're fired. |
| 0 - 5.0 | Survived. Barely. |
| 5.0 - 20.0 | Solid P&L. Promoted! |
| > 20.0 | LEGEND. Partner track! |

### Tips

- Keep your position small; large positions have large unrealized swings
- Widen your spread when volatility is high (hard mode)
- Watch Theta: your option decays each round
- Use manual Buy/Sell to cut losing positions before they grow

## Technical Notes

- All prices stored as fixed-point x10 (e.g. 105.5 = 1055)
- Greeks: Delta x100, Gamma x1000, Theta x10, Vega x10
- Normal CDF computed via 21-entry lookup table with linear interpolation
- XOR constructed from AND/OR/NOT (Jack has no XOR operator)
- Newton-Raphson integer square root with 10-iteration limit
- Per-tick string allocations eliminated via pre-allocated field strings and printChar
- Tick delay (default 200ms) is parametrizable via `HackTrader.setTickDelay(ms)`.
  On actual Hack hardware (~50K instructions/sec), increase to 500-1000ms for smooth gameplay.
- Game duration (default 30 rounds) is parametrizable via `HackTrader.setTotalRounds(n)`.
  Call these setters after `HackTrader.init()` but before `HackTrader.new()`.

## Author

Žarko Gvozdenović (zarko@visaurum.nl) — [Visaurum](https://www.linkedin.com/company/visaurum-b-v/)

## License

MIT License. See [LICENSE](LICENSE) for details. Part of nand2tetris course materials.
