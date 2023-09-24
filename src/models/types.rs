use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum OrderType {
    OrderTypeLimit,
    OrderTypeMarket,
}

impl ToString for OrderType {
    fn to_string(&self) -> String {
        return match self {
            OrderType::OrderTypeLimit => "limit".to_string(),
            OrderType::OrderTypeMarket => "market".to_string(),
        };
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Side {
    SideBuy,
    SideSell,
}

impl Side {
    pub fn opposite(self) -> Self {
        return match self {
            Side::SideBuy => Side::SideSell,
            Side::SideSell => Side::SideBuy,
        };
    }
}

impl ToString for Side {
    fn to_string(&self) -> String {
        return match self {
            Side::SideBuy => "buy".to_string(),
            Side::SideSell => "sell".to_string(),
        };
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum TimeInForceType {
    GoodTillCanceled,
    ImmediateOrCancel,
    GoodTillCrossing,
    FillOrKill,
}

impl ToString for TimeInForceType {
    fn to_string(&self) -> String {
        return match self {
            TimeInForceType::GoodTillCanceled => "GTC".to_string(),
            TimeInForceType::ImmediateOrCancel => "IOC".to_string(),
            TimeInForceType::GoodTillCrossing => "GTX".to_string(),
            TimeInForceType::FillOrKill => "FOK".to_string(),
        };
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum OrderStatus {
    OrderStatusNew,
    OrderStatusOpen,
    OrderStatusCancelling,
    OrderStatusCancelled,
    OrderStatusPartial,
    OrderStatusFilled,
}

impl ToString for OrderStatus {
    fn to_string(&self) -> String {
        return match self {
            OrderStatus::OrderStatusNew => "new".to_string(),
            OrderStatus::OrderStatusOpen => "open".to_string(),
            OrderStatus::OrderStatusCancelling => "cancelling".to_string(),
            OrderStatus::OrderStatusCancelled => "cancelled".to_string(),
            OrderStatus::OrderStatusPartial => "partial".to_string(),
            OrderStatus::OrderStatusFilled => "filled".to_string(),
        };
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum DoneReason {
    DoneReasonFilled,
    DoneReasonCancelled,
}

impl ToString for DoneReason {
    fn to_string(&self) -> String {
        return match self {
            DoneReason::DoneReasonFilled => "filled".to_string(),
            DoneReason::DoneReasonCancelled => "cancelled".to_string(),
        };
    }
}

pub const ORDER_TYPE_LIMIT: OrderType = OrderType::OrderTypeLimit;
pub const ORDER_TYPE_MARKET: OrderType = OrderType::OrderTypeMarket;

pub const SIDE_BUY: Side = Side::SideBuy;
pub const SIDE_SELL: Side = Side::SideSell;

pub const GOOD_TILL_CANCELED: TimeInForceType = TimeInForceType::GoodTillCanceled;
pub const IMMEDIATE_OR_CANCEL: TimeInForceType = TimeInForceType::ImmediateOrCancel;
pub const GOOD_TILL_CROSSING: TimeInForceType = TimeInForceType::GoodTillCrossing;
pub const FILL_OR_KILL: TimeInForceType = TimeInForceType::FillOrKill;

pub const ORDER_STATUS_NEW: OrderStatus = OrderStatus::OrderStatusNew;
pub const ORDER_STATUS_OPEN: OrderStatus = OrderStatus::OrderStatusOpen;
pub const ORDER_STATUS_CANCELLING: OrderStatus = OrderStatus::OrderStatusCancelling;
pub const ORDER_STATUS_CANCELLED: OrderStatus = OrderStatus::OrderStatusCancelled;
pub const ORDER_STATUS_PARTIAL: OrderStatus = OrderStatus::OrderStatusPartial;
pub const ORDER_STATUS_FILLED: OrderStatus = OrderStatus::OrderStatusFilled;

pub const DONE_REASON_FILLED: DoneReason = DoneReason::DoneReasonFilled;
pub const DONE_REASON_CANCELLED: DoneReason = DoneReason::DoneReasonCancelled;
