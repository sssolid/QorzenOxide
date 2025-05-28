// src/ui/mod.rs
use dioxus::prelude::*;
use qorzen_oxide::{
    ui::{UIComponent, UIComponentType},
    types::Permission,
};

pub struct ProductCatalogUI;

impl ProductCatalogUI {
    pub fn render_main_page(cx: Scope) -> Element {
        let products = use_state(cx, || Vec::<Product>::new());
        let loading = use_state(cx, || true);
        let search_term = use_state(cx, String::new);
        let selected_category = use_state(cx, || None::<String>);

        // Load products on component mount
        use_effect(cx, (), |_| {
            to_owned![products, loading];
            async move {
                if let Ok(product_list) = fetch_products().await {
                    products.set(product_list);
                    loading.set(false);
                }
            }
        });

        render! {
            div { class: "product-catalog-page",
                div { class: "page-header",
                    h1 { "Product Catalog" }
                    div { class: "header-actions",
                        button { 
                            class: "btn btn-primary",
                            onclick: |_| {
                                // Handle add product
                            },
                            "Add Product"
                        }
                        button { 
                            class: "btn btn-secondary",
                            onclick: |_| {
                                // Handle import products
                            },
                            "Import Products"
                        }
                    }
                }
                
                div { class: "product-filters",
                    div { class: "search-bar",
                        input {
                            r#type: "text",
                            placeholder: "Search products...",
                            value: "{search_term}",
                            oninput: move |evt| search_term.set(evt.value.clone()),
                        }
                    }
                    
                    div { class: "category-filter",
                        select {
                            value: "{selected_category:?}",
                            onchange: move |evt| {
                                selected_category.set(
                                    if evt.value.is_empty() { 
                                        None 
                                    } else { 
                                        Some(evt.value.clone()) 
                                    }
                                );
                            },
                            option { value: "", "All Categories" }
                            // Render category options dynamically
                        }
                    }
                }
                
                if **loading {
                    div { class: "loading-spinner",
                        "Loading products..."
                    }
                } else {
                    div { class: "product-grid",
                        products.iter().map(|product| rsx! {
                            ProductCard { 
                                key: "{product.id}",
                                product: product.clone(),
                                on_edit: |product_id| {
                                    // Handle edit product
                                },
                                on_delete: |product_id| {
                                    // Handle delete product
                                }
                            }
                        })
                    }
                }
                
                // Pagination component
                div { class: "pagination",
                    // Pagination controls
                }
            }
        }
    }

    pub fn render_quick_add_panel(cx: Scope) -> Element {
        let product_name = use_state(cx, String::new);
        let product_price = use_state(cx, String::new);
        let product_category = use_state(cx, String::new);
        let is_submitting = use_state(cx, || false);

        render! {
            div { class: "quick-add-panel",
                h3 { "Quick Add Product" }
                
                form {
                    onsubmit: move |evt| {
                        evt.prevent_default();
                        if !**is_submitting {
                            is_submitting.set(true);
                            // Handle form submission
                        }
                    },
                    
                    div { class: "form-group",
                        label { "Product Name" }
                        input {
                            r#type: "text",
                            value: "{product_name}",
                            oninput: move |evt| product_name.set(evt.value.clone()),
                            required: true,
                        }
                    }
                    
                    div { class: "form-group",
                        label { "Price" }
                        input {
                            r#type: "number",
                            step: "0.01",
                            value: "{product_price}",
                            oninput: move |evt| product_price.set(evt.value.clone()),
                            required: true,
                        }
                    }
                    
                    div { class: "form-group",
                        label { "Category" }
                        select {
                            value: "{product_category}",
                            onchange: move |evt| product_category.set(evt.value.clone()),
                            required: true,
                            // Category options
                        }
                    }
                    
                    div { class: "form-actions",
                        button {
                            r#type: "submit",
                            class: "btn btn-primary",
                            disabled: **is_submitting,
                            if **is_submitting { "Adding..." } else { "Add Product" }
                        }
                        button {
                            r#type: "button",
                            class: "btn btn-secondary",
                            onclick: |_| {
                                // Reset form
                            },
                            "Reset"
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn ProductCard(cx: Scope, product: Product, on_edit: EventHandler<String>, on_delete: EventHandler<String>) -> Element {
    render! {
        div { class: "product-card",
            div { class: "product-image",
                if let Some(image_url) = &product.image_url {
                    img { src: "{image_url}", alt: "{product.name}" }
                } else {
                    div { class: "no-image", "No Image" }
                }
            }
            
            div { class: "product-info",
                h3 { class: "product-name", "{product.name}" }
                p { class: "product-description", "{product.description}" }
                div { class: "product-price", "${product.price}" }
                
                if let Some(category) = &product.category {
                    div { class: "product-category", 
                        span { class: "category-badge", "{category.name}" }
                    }
                }
            }
            
            div { class: "product-actions",
                button {
                    class: "btn btn-sm btn-primary",
                    onclick: move |_| on_edit.call(product.id.clone()),
                    "Edit"
                }
                button {
                    class: "btn btn-sm btn-danger",
                    onclick: move |_| on_delete.call(product.id.clone()),
                    "Delete"
                }
            }
            
            if product.inventory.quantity <= product.inventory.low_stock_threshold {
                div { class: "stock-warning",
                    "⚠️ Low Stock: {product.inventory.quantity} remaining"
                }
            }
        }
    }
}

async fn fetch_products() -> Result<Vec<Product>, Error> {
    // Implementation to fetch products from API
    // This would use the plugin's API client
    todo!()
}