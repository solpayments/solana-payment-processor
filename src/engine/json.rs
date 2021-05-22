use serde::{Deserialize, Serialize};

#[derive(Serialize, Debug, Deserialize, PartialEq)]
/// Subscription package
pub struct Package {
    pub name: String,
    /// duration of the subscription in seconds
    pub duration: i64,
    /// the price in full for this subscription option
    /// e.g. if the duration is 1 hour (3600) then the price is per hour
    /// e.g. if the duration is 1 month (3600 * 24 * 30) then the price is per month
    pub price: u64,
}

#[derive(Serialize, Debug, Deserialize, PartialEq)]
/// Subscription packages
pub struct Packages {
    pub packages: Vec<Package>,
}

#[derive(Serialize, Debug, Deserialize, PartialEq)]
/// Used in order account data field to tie the order to a subscription
pub struct OrderSubscription {
    pub subscription: String,
}