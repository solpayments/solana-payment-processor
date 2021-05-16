use serde::{Deserialize, Serialize};

#[derive(Serialize, Debug, Deserialize, PartialEq)]
pub struct Package {
    pub name: String,
    pub duration: i64,
    pub price: u64,
}

#[derive(Serialize, Debug, Deserialize, PartialEq)]
pub struct Packages {
    pub packages: Vec<Package>,
}