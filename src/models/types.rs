use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum OrderType {
    OrderTypeLimit,
    OrderTypeMarket,
}

pub fn serialize_order_type<S>(order_type: &OrderType, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let string = match order_type {
        OrderType::OrderTypeLimit => "limit",
        OrderType::OrderTypeMarket => "market",
    };
    serializer.serialize_str(string)
}

pub fn deserialize_order_type<'de, D>(deserializer: D) -> Result<OrderType, D::Error>
where
    D: Deserializer<'de>,
{
    let string: &str = Deserialize::deserialize(deserializer)?;
    match string {
        "limit" => Ok(OrderType::OrderTypeLimit),
        "market" => Ok(OrderType::OrderTypeMarket),
        _ => Err(serde::de::Error::custom("invalid order_type string")),
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

pub fn serialize_side<S>(side: &Side, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let string = match side {
        Side::SideBuy => "buy",
        Side::SideSell => "sell",
    };
    serializer.serialize_str(string)
}

pub fn deserialize_side<'de, D>(deserializer: D) -> Result<Side, D::Error>
where
    D: Deserializer<'de>,
{
    let string: &str = Deserialize::deserialize(deserializer)?;
    match string {
        "buy" => Ok(Side::SideBuy),
        "sell" => Ok(Side::SideSell),
        _ => Err(serde::de::Error::custom("invalid side string")),
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum TimeInForceType {
    GoodTillCanceled,
    ImmediateOrCancel,
    GoodTillCrossing,
    FillOrKill,
}

pub fn serialize_time_in_force_type<S>(
    time_in_force_type: &TimeInForceType,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let string = match time_in_force_type {
        TimeInForceType::GoodTillCanceled => "GTC",
        TimeInForceType::ImmediateOrCancel => "IOC",
        TimeInForceType::GoodTillCrossing => "GTX",
        TimeInForceType::FillOrKill => "FOK",
    };
    serializer.serialize_str(string)
}

pub fn deserialize_time_in_force_type<'de, D>(deserializer: D) -> Result<TimeInForceType, D::Error>
where
    D: Deserializer<'de>,
{
    let string: &str = Deserialize::deserialize(deserializer)?;
    match string {
        "GTC" => Ok(TimeInForceType::GoodTillCanceled),
        "IOC" => Ok(TimeInForceType::ImmediateOrCancel),
        "GTX" => Ok(TimeInForceType::GoodTillCrossing),
        "FOK" => Ok(TimeInForceType::FillOrKill),
        _ => Err(serde::de::Error::custom(
            "invalid time_in_force_type string",
        )),
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

pub fn serialize_order_status<S>(
    order_status: &OrderStatus,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let string = match order_status {
        OrderStatus::OrderStatusNew => "new",
        OrderStatus::OrderStatusOpen => "open",
        OrderStatus::OrderStatusCancelling => "cancelling",
        OrderStatus::OrderStatusCancelled => "cancelled",
        OrderStatus::OrderStatusPartial => "partial",
        OrderStatus::OrderStatusFilled => "filled",
    };
    serializer.serialize_str(string)
}

pub fn deserialize_order_status<'de, D>(deserializer: D) -> Result<OrderStatus, D::Error>
where
    D: Deserializer<'de>,
{
    let string: &str = Deserialize::deserialize(deserializer)?;
    match string {
        "new" => Ok(OrderStatus::OrderStatusNew),
        "open" => Ok(OrderStatus::OrderStatusOpen),
        "cancelling" => Ok(OrderStatus::OrderStatusCancelling),
        "cancelled" => Ok(OrderStatus::OrderStatusCancelled),
        "partial" => Ok(OrderStatus::OrderStatusPartial),
        "filled" => Ok(OrderStatus::OrderStatusFilled),
        _ => Err(serde::de::Error::custom("invalid order_status string")),
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum DoneReason {
    DoneReasonFilled,
    DoneReasonCancelled,
}

pub fn serialize_done_reason<S>(done_reason: &DoneReason, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let string = match done_reason {
        DoneReason::DoneReasonFilled => "filled",
        DoneReason::DoneReasonCancelled => "cancelled",
    };
    serializer.serialize_str(string)
}

pub fn deserialize_done_reason<'de, D>(deserializer: D) -> Result<DoneReason, D::Error>
where
    D: Deserializer<'de>,
{
    let string: &str = Deserialize::deserialize(deserializer)?;
    match string {
        "filled" => Ok(DoneReason::DoneReasonFilled),
        "cancelled" => Ok(DoneReason::DoneReasonCancelled),
        _ => Err(serde::de::Error::custom("invalid done_reason string")),
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
