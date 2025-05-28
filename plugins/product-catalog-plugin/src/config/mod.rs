// src/config/mod.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductCatalogConfig {
    pub default_currency: String,
    pub enable_inventory_tracking: bool,
    pub low_stock_threshold: u32,
    pub enable_barcode_scanning: bool,
    pub image_upload_max_size_mb: u32,
    pub supported_image_formats: Vec<String>,
    pub enable_product_reviews: bool,
    pub enable_product_variants: bool,
    pub tax_calculation_mode: TaxCalculationMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaxCalculationMode {
    Inclusive,
    Exclusive,
    None,
}

impl Default for ProductCatalogConfig {
    fn default() -> Self {
        Self {
            default_currency: "USD".to_string(),
            enable_inventory_tracking: true,
            low_stock_threshold: 10,
            enable_barcode_scanning: false,
            image_upload_max_size_mb: 5,
            supported_image_formats: vec![
                "image/jpeg".to_string(),
                "image/png".to_string(),
                "image/webp".to_string(),
            ],
            enable_product_reviews: false,
            enable_product_variants: false,
            tax_calculation_mode: TaxCalculationMode::Exclusive,
        }
    }
}