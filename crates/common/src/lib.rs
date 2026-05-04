pub type Price = u32; // ITCH prices are 4-byte integers (Price 4)
pub type Quantity = u32; // ITCH quantities are 4-byte integers
pub type OrderId = u64; // ITCH Order Reference Numbers are 8 bytes

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Buy,
    Sell,
}

#[repr(i8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AggressorSide {
    Buyer,
    Seller,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum OrderSide {
    Buy = 0,
    Sell = 1,
}
