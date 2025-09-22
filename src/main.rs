use std::collections::{BTreeMap, VecDeque};
use std::io::{self, Write};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Buy,
    Sell,
}

impl fmt::Display for Side {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Side::Buy => write!(f, "BUY"),
            Side::Sell => write!(f, "SELL"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Order {
    pub id: u64,
    pub side: Side,
    pub price: i64,
    pub quantity: i64,
    pub timestamp: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Trade {
    pub price: i64,
    pub quantity: i64,
    pub maker_id: u64,
    pub taker_id: u64,
}

#[derive(Debug, Default)]
pub struct OrderBook {
    buy_levels: BTreeMap<i64, VecDeque<Order>>,
    sell_levels: BTreeMap<i64, VecDeque<Order>>,
    next_timestamp: u64,
    symbol: String,
}

impl OrderBook {
    pub fn new(symbol: String) -> Self {
        Self {
            buy_levels: BTreeMap::new(),
            sell_levels: BTreeMap::new(),
            next_timestamp: 1,
            symbol,
        }
    }

    pub fn place_order(&mut self, side: Side, price: i64, quantity: i64, id: u64) -> Vec<Trade> {
        let mut trades = Vec::new();
        let mut remaining_qty = quantity;
        let timestamp = self.next_timestamp;
        self.next_timestamp += 1;

        match side {
            Side::Buy => {
                while remaining_qty > 0 {
                    let best_sell_price = match self.sell_levels.keys().next() {
                        Some(&p) => p,
                        None => break,
                    };

                    if price < best_sell_price {
                        break;
                    }

                    let mut level_empty = false;
                    if let Some(orders) = self.sell_levels.get_mut(&best_sell_price) {
                        while let Some(mut resting_order) = orders.pop_front() {
                            let trade_qty = std::cmp::min(remaining_qty, resting_order.quantity);
                            
                            trades.push(Trade {
                                price: resting_order.price,
                                quantity: trade_qty,
                                maker_id: resting_order.id,
                                taker_id: id,
                            });

                            remaining_qty -= trade_qty;
                            resting_order.quantity -= trade_qty;

                            if resting_order.quantity > 0 {
                                orders.push_front(resting_order);
                                break;
                            }

                            if remaining_qty == 0 {
                                break;
                            }
                        }

                        level_empty = orders.is_empty();
                    }

                    if level_empty {
                        self.sell_levels.remove(&best_sell_price);
                    }

                    if remaining_qty == 0 {
                        break;
                    }
                }
            }
            Side::Sell => {
                while remaining_qty > 0 {
                    let best_buy_price = match self.buy_levels.keys().next_back() {
                        Some(&p) => p,
                        None => break,
                    };

                    if price > best_buy_price {
                        break;
                    }

                    let mut level_empty = false;
                    if let Some(orders) = self.buy_levels.get_mut(&best_buy_price) {
                        while let Some(mut resting_order) = orders.pop_front() {
                            let trade_qty = std::cmp::min(remaining_qty, resting_order.quantity);
                            
                            trades.push(Trade {
                                price: resting_order.price,
                                quantity: trade_qty,
                                maker_id: resting_order.id,
                                taker_id: id,
                            });

                            remaining_qty -= trade_qty;
                            resting_order.quantity -= trade_qty;

                            if resting_order.quantity > 0 {
                                orders.push_front(resting_order);
                                break;
                            }

                            if remaining_qty == 0 {
                                break;
                            }
                        }

                        level_empty = orders.is_empty();
                    }

                    if level_empty {
                        self.buy_levels.remove(&best_buy_price);
                    }

                    if remaining_qty == 0 {
                        break;
                    }
                }
            }
        }

        if remaining_qty > 0 {
            let remaining_order = Order {
                id,
                side,
                price,
                quantity: remaining_qty,
                timestamp,
            };

            match side {
                Side::Buy => {
                    self.buy_levels
                        .entry(price)
                        .or_insert_with(VecDeque::new)
                        .push_back(remaining_order);
                }
                Side::Sell => {
                    self.sell_levels
                        .entry(price)
                        .or_insert_with(VecDeque::new)
                        .push_back(remaining_order);
                }
            }
        }

        trades
    }

    pub fn best_buy(&self) -> Option<(i64, i64)> {
        self.buy_levels
            .iter()
            .next_back()
            .map(|(price, orders)| {
                let total_quantity = orders.iter().map(|o| o.quantity).sum();
                (*price, total_quantity)
            })
    }

    pub fn best_sell(&self) -> Option<(i64, i64)> {
        self.sell_levels
            .iter()
            .next()
            .map(|(price, orders)| {
                let total_quantity = orders.iter().map(|o| o.quantity).sum();
                (*price, total_quantity)
            })
    }

    pub fn display_book(&self, depth: usize) {
        println!("\n🚀 {} ORDER BOOK", self.symbol);
        println!("═══════════════════════════════════════");
        
        // Get sell levels (reversed to show highest first)
        let mut sell_levels: Vec<_> = self.sell_levels.iter().collect();
        sell_levels.reverse();
        
        // Display top sell levels
        println!("📈 ASK SIDE (SELL ORDERS):");
        for (price, orders) in sell_levels.iter().take(depth) {
            let total_qty: i64 = orders.iter().map(|o| o.quantity).sum();
            let num_orders = orders.len();
            let price_val = **price;
            println!("   ${:>7.2} │ {:>8.4} Valhalla │ {} orders", 
                price_val as f64 / 100.0, total_qty as f64 / 10000.0, num_orders);
        }
        
        // Show spread
        let spread = match (self.best_sell(), self.best_buy()) {
            (Some((ask, _)), Some((bid, _))) => format!("${:.2}", (ask - bid) as f64 / 100.0),
            _ => "N/A".to_string(),
        };
        println!("         ├─ SPREAD: {} ─┤", spread);
        
        // Display top buy levels  
        println!("📉 BID SIDE (BUY ORDERS):");
        for (price, orders) in self.buy_levels.iter().rev().take(depth) {
            let total_qty: i64 = orders.iter().map(|o| o.quantity).sum();
            let num_orders = orders.len();
            let price_val = *price;
            println!("   ${:>7.2} │ {:>8.4} Valhalla │ {} orders", 
                price_val as f64 / 100.0, total_qty as f64 / 10000.0, num_orders);
        }
        
        println!("═══════════════════════════════════════");
    }
}

struct TradingEngine {
    book: OrderBook,
    next_order_id: u64,
    trades_history: Vec<(Trade, String)>, // Trade + timestamp
}

impl TradingEngine {
    fn new() -> Self {
        Self {
            book: OrderBook::new("Valhalla/USD".to_string()),
            next_order_id: 1000,
            trades_history: Vec::new(),
        }
    }

    fn place_order(&mut self, side: Side, price: f64, quantity: f64) -> Result<Vec<Trade>, String> {
        // Convert to integer representation (price in cents, quantity in 0.0001 units)
        let price_int = (price * 100.0) as i64;
        let quantity_int = (quantity * 10000.0) as i64;
        
        if quantity_int <= 0 || price_int <= 0 {
            return Err("Price and quantity must be positive".to_string());
        }

        let order_id = self.next_order_id;
        self.next_order_id += 1;

        println!("\n⚡ INCOMING ORDER:");
        println!("   Order #{}: {} {:.4} Valhalla @ ${:.2}", 
            order_id, side, quantity, price);

        let trades = self.book.place_order(side, price_int, quantity_int, order_id);

        if !trades.is_empty() {
            println!("\n🎯 TRADES EXECUTED:");
            for (i, trade) in trades.iter().enumerate() {
                let trade_price = trade.price as f64 / 100.0;
                let trade_qty = trade.quantity as f64 / 10000.0;
                let trade_value = trade_price * trade_qty;
                
                println!("   Trade #{}: {:.4} Valhalla @ ${:.2} = ${:.2} (Maker: #{}, Taker: #{})",
                    i + 1, trade_qty, trade_price, trade_value, trade.maker_id, trade.taker_id);
                
                // Add to trades history with timestamp
                let timestamp = format!("{:02}:{:02}:{:02}", 
                    (self.trades_history.len() / 3600) % 24,
                    (self.trades_history.len() / 60) % 60,
                    self.trades_history.len() % 60);
                self.trades_history.push((trade.clone(), timestamp));
            }
        } else {
            println!("   ➕ Order added to book (no matches)");
        }

        Ok(trades)
    }

    fn display_main_view(&self) {
        // Clear screen
        print!("\x1B[2J\x1B[1;1H");
        
        println!("🚀 VALHALLA TRADING ENGINE");
        println!("═══════════════════════════════════════════════════════════════════════");
        
        // Display order book and time & sales side by side
        self.display_book_and_sales();
    }

    fn display_book_and_sales(&self) {
        // Get order book lines
        let mut book_lines = Vec::new();
        
        book_lines.push(format!("📊 ORDER BOOK"));
        book_lines.push(format!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"));
        
        // Get sell levels (reversed to show highest first)
        let mut sell_levels: Vec<_> = self.book.sell_levels.iter().collect();
        sell_levels.reverse();
        
        // ASK side
        book_lines.push(format!("📈 ASK SIDE:"));
        for (price, orders) in sell_levels.iter().take(5) {
            let total_qty: i64 = orders.iter().map(|o| o.quantity).sum();
            let num_orders = orders.len();
            let price_val = **price;
            book_lines.push(format!("${:>7.2} │ {:>8.4} │ {} orders", 
                price_val as f64 / 100.0, total_qty as f64 / 10000.0, num_orders));
        }
        
        // Spread
        let spread = match (self.book.best_sell(), self.book.best_buy()) {
            (Some((ask, _)), Some((bid, _))) => format!("${:.2}", (ask - bid) as f64 / 100.0),
            _ => "N/A".to_string(),
        };
        book_lines.push(format!("      ├─ SPREAD: {} ─┤", spread));
        
        // BID side
        book_lines.push(format!("📉 BID SIDE:"));
        for (price, orders) in self.book.buy_levels.iter().rev().take(5) {
            let total_qty: i64 = orders.iter().map(|o| o.quantity).sum();
            let num_orders = orders.len();
            let price_val = *price;
            book_lines.push(format!("${:>7.2} │ {:>8.4} │ {} orders", 
                price_val as f64 / 100.0, total_qty as f64 / 10000.0, num_orders));
        }

        // Get time & sales lines
        let mut sales_lines = Vec::new();
        sales_lines.push(format!("📈 TIME & SALES"));
        sales_lines.push(format!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"));
        sales_lines.push(format!("Time     │ Price    │ Size     │ Side"));
        sales_lines.push(format!("─────────┼──────────┼──────────┼─────"));
        
        // Show last 10 trades
        for (trade, timestamp) in self.trades_history.iter().rev().take(10) {
            let price = trade.price as f64 / 100.0;
            let qty = trade.quantity as f64 / 10000.0;
            sales_lines.push(format!("{}   │ ${:>7.2} │ {:>8.4} │ FILL", 
                timestamp, price, qty));
        }
        
        if self.trades_history.is_empty() {
            sales_lines.push(format!("No trades yet..."));
        }

        // Print side by side
        let max_lines = std::cmp::max(book_lines.len(), sales_lines.len());
        let empty_string = String::new();
        
        for i in 0..max_lines {
            let left = book_lines.get(i).map(|s| format!("{:<35}", s)).unwrap_or_else(|| " ".repeat(35));
            let right = sales_lines.get(i).unwrap_or(&empty_string);
            println!("{}│ {}", left, right);
        }
        
        println!("═══════════════════════════════════════════════════════════════════════");
    }

    fn seed_market_data(&mut self) {
        println!("🌱 Seeding Valhalla market with initial orders...\n");

        // Valhalla/USD set around $1000
        let valcoin_orders = vec![
            (Side::Buy, 995.0, 15.0),
            (Side::Buy, 990.0, 25.0), 
            (Side::Buy, 985.0, 35.0),
            (Side::Buy, 980.0, 45.0),
            (Side::Sell, 1005.0, 20.0),
            (Side::Sell, 1010.0, 30.0),
            (Side::Sell, 1015.0, 40.0),
            (Side::Sell, 1020.0, 50.0),
        ];

        for (side, price, qty) in valcoin_orders {
            let _ = self.place_order(side, price, qty);
        }
    }
}

fn get_input(prompt: &str) -> String {
    print!("{}", prompt);
    io::stdout().flush().unwrap();
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    input.trim().to_string()
}

fn parse_side(input: &str) -> Result<Side, String> {
    match input.to_lowercase().as_str() {
        "buy" | "b" => Ok(Side::Buy),
        "sell" | "s" => Ok(Side::Sell),
        _ => Err("Please enter 'buy'/'b' or 'sell'/'s'".to_string()),
    }
}

fn main() {
    let mut engine = TradingEngine::new();
    engine.seed_market_data();
    
    loop {
        engine.display_main_view();
        
        println!("\n🎯 MAIN MENU");
        println!("1. 💹 Place Order");
        println!("2. 🚪 Exit");
        
        let choice = get_input("\nChoose option (1-2): ");
        
        match choice.as_str() {
            "1" => {
                println!("\n💹 PLACE NEW ORDER");
                
                let side_input = get_input("Side (buy/sell): ");
                let side = match parse_side(&side_input) {
                    Ok(s) => s,
                    Err(e) => {
                        println!("❌ {}", e);
                        get_input("\nPress Enter to continue...");
                        continue;
                    }
                };
                
                let price_input = get_input("Price ($): ");
                let price: f64 = match price_input.parse() {
                    Ok(p) => p,
                    Err(_) => {
                        println!("❌ Invalid price!");
                        get_input("\nPress Enter to continue...");
                        continue;
                    }
                };
                
                let qty_input = get_input("Quantity: ");
                let quantity: f64 = match qty_input.parse() {
                    Ok(q) => q,
                    Err(_) => {
                        println!("❌ Invalid quantity!");
                        get_input("\nPress Enter to continue...");
                        continue;
                    }
                };
                
                match engine.place_order(side, price, quantity) {
                    Ok(_) => {
                        get_input("\nPress Enter to continue...");
                    },
                    Err(e) => {
                        println!("❌ Error: {}", e);
                        get_input("\nPress Enter to continue...");
                    }
                }
            },
            "2" => {
                println!("👋 Thanks for using the Valhalla Trading Engine!");
                break;
            },
            _ => {
                println!("❌ Invalid choice, please try again.");
                get_input("\nPress Enter to continue...");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valcoin_trading_engine() {
        let mut engine = TradingEngine::new();
        
        // Test placing orders
        let trades = engine.place_order(Side::Buy, 1000.0, 10.0).unwrap();
        assert!(trades.is_empty()); // No matching orders
        
        let trades = engine.place_order(Side::Sell, 1000.0, 5.0).unwrap();
        assert_eq!(trades.len(), 1); // Should match
        assert_eq!(trades[0].quantity, 50000); // 5.0 * 10000
    }

    #[test]
    fn test_valcoin_pricing() {
        let mut book = OrderBook::new("Valhalla/USD".to_string());
        
        // Test Valhalla prices around $1000
        let trades = book.place_order(Side::Buy, 100000, 100000, 1); // $1000, 10.0 Valhalla
        assert!(trades.is_empty());
        
        let trades = book.place_order(Side::Sell, 100000, 50000, 2); // $1000, 5.0 Valhalla  
        assert_eq!(trades.len(), 1);
        assert_eq!(trades[0].price, 100000); // $1000.00
    }

    #[test]
    fn test_time_and_sales() {
        let mut engine = TradingEngine::new();
        
        // Place orders that will trade
        engine.place_order(Side::Buy, 1000.0, 10.0).unwrap();
        let trades = engine.place_order(Side::Sell, 1000.0, 5.0).unwrap();
        
        assert_eq!(trades.len(), 1);
        assert_eq!(engine.trades_history.len(), 1);
    }
}