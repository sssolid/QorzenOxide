// src/models/mod.rs
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Product {
    pub id: String,
    pub name: String,
    pub description: String,
    pub price: f64,
    pub sku: Option<String>,
    pub barcode: Option<String>,
    pub category: Option<Category>,
    pub image_url: Option<String>,
    pub weight: Option<f64>,
    pub dimensions: Option<Dimensions>,
    pub inventory: Inventory,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub created_by: Option<String>,
    pub updated_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Category {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub parent_id: Option<String>,
    pub sort_order: i32,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Inventory {
    pub product_id: String,
    pub quantity: u32,
    pub reserved_quantity: u32,
    pub low_stock_threshold: u32,
    pub reorder_point: u32,
    pub reorder_quantity: u32,
    pub cost: Option<f64>,
    pub last_restocked_at: Option<DateTime<Utc>>,
    pub last_sold_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dimensions {
    pub length: f64,
    pub width: f64,
    pub height: f64,
    pub unit: String, // "cm", "in", etc.
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductVariant {
    pub id: String,
    pub product_id: String,
    pub name: String,
    pub sku: Option<String>,
    pub price_adjustment: f64, // Amount to add/subtract from base price
    pub weight_adjustment: Option<f64>,
    pub attributes: std::collections::HashMap<String, String>, // color: "red", size: "large"
    pub inventory: Inventory,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductReview {
    pub id: String,
    pub product_id: String,
    pub user_id: String,
    pub rating: u8, // 1-5 stars
    pub title: String,
    pub comment: String,
    pub is_verified_purchase: bool,
    pub is_approved: bool,
    pub helpful_votes: u32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}