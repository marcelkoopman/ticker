use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub assets: Vec<Asset>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Asset {
    pub name: String,
    pub url: String,
    pub price_path: String,
    pub unit: String,
}

#[derive(Debug, Clone)]
pub struct Price {
    pub name: String,
    pub value: f64,
    pub unit: String,
}
