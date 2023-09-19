use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Serialize, Deserialize, Clone)]
pub enum OrderType {
    #[default]
    OrderTypeLimit,
    OrderTypeMarket,
}

#[derive(Default, Debug, Serialize, Deserialize, Clone)]
pub enum Side {
    #[default]
    SideBuy,
    SideSell,
}

#[derive(Default, Debug, Serialize, Deserialize, Clone)]
pub enum TimeInForceType {
    #[default]
    GoodTillCanceled,
    ImmediateOrCancel,
    GoodTillCrossing,
    FillOrKill,
}

#[derive(Default, Debug, Serialize, Deserialize, Clone)]
pub enum OrderStatus {
    #[default]
    OrderStatusNew,
    OrderStatusOpen,
    OrderStatusCancelling,
    OrderStatusCancelled,
    OrderStatusPartial,
    OrderStatusFilled,
}

#[derive(Default, Debug, Serialize, Deserialize, Clone)]
pub enum DoneReason {
    #[default]
    DoneReasonFilled,
    DoneReasonCancelled,
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
