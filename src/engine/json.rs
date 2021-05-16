use serde::{Deserialize, Serialize};

#[derive(Serialize, Debug, Deserialize, PartialEq)]
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
pub struct Packages {
    pub packages: Vec<Package>,
}