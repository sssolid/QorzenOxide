pub mod plugin_service;

pub use plugin_service::{PluginService, PluginServiceProvider, use_plugin_service, get_plugin_service, initialize_plugin_service};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_exports() {
        // Test that module exports are accessible
        let _service = PluginService::new();
        let _global_service = get_plugin_service();
    }
}