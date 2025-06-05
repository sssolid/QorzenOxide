// src/app/debug.rs
use std::time::{Duration, Instant};
use tokio::time::timeout;
use tracing::{error, info, warn};

/// Debug-friendly application initializer with proper error handling and timeouts
pub struct DebugApplicationCore {
    initialization_steps: Vec<InitStep>,
    current_step: usize,
    start_time: Instant,
}

#[derive(Debug, Clone)]
pub struct InitStep {
    pub name: &'static str,
    pub timeout: Duration,
    pub optional: bool,
}

impl DebugApplicationCore {
    pub fn new() -> Self {
        // Setup console error handling first
        #[cfg(target_arch = "wasm32")]
        {
            console_error_panic_hook::set_once();
            // Force console logging to work
            web_sys::console::log_1(&"🚀 Debug Application Core Starting".into());
        }

        Self {
            initialization_steps: vec![
                InitStep { name: "logging", timeout: Duration::from_secs(5), optional: false },
                InitStep { name: "platform", timeout: Duration::from_secs(10), optional: false },
                InitStep { name: "config", timeout: Duration::from_secs(5), optional: true },
                InitStep { name: "event_bus", timeout: Duration::from_secs(5), optional: true },
                InitStep { name: "plugin_registry", timeout: Duration::from_secs(10), optional: true },
                InitStep { name: "ui", timeout: Duration::from_secs(5), optional: false },
            ],
            current_step: 0,
            start_time: Instant::now(),
        }
    }

    pub async fn initialize_with_debug(&mut self) -> Result<MinimalApp, InitError> {
        self.setup_emergency_logging().await;

        info!("🚀 Starting debug application initialization");

        let mut app = MinimalApp::new();

        for (index, step) in self.initialization_steps.iter().enumerate() {
            self.current_step = index;

            let step_start = Instant::now();
            info!("📋 Step {}/{}: Initializing {}", index + 1, self.initialization_steps.len(), step.name);

            let result = timeout(step.timeout, self.run_init_step(step.name, &mut app)).await;

            match result {
                Ok(Ok(())) => {
                    let duration = step_start.elapsed();
                    info!("✅ Step {}: {} completed in {:?}", index + 1, step.name, duration);
                }
                Ok(Err(e)) => {
                    let duration = step_start.elapsed();
                    if step.optional {
                        warn!("⚠️  Step {}: {} failed (optional) in {:?}: {}", index + 1, step.name, duration, e);
                    } else {
                        error!("❌ Step {}: {} failed (required) in {:?}: {}", index + 1, step.name, duration, e);
                        return Err(InitError::StepFailed {
                            step: step.name.to_string(),
                            error: e.to_string(),
                            duration,
                        });
                    }
                }
                Err(_) => {
                    error!("⏰ Step {}: {} timed out after {:?}", index + 1, step.name, step.timeout);
                    if !step.optional {
                        return Err(InitError::Timeout {
                            step: step.name.to_string(),
                            timeout: step.timeout
                        });
                    }
                }
            }
        }

        let total_duration = self.start_time.elapsed();
        info!("🎉 Application initialization completed in {:?}", total_duration);

        Ok(app)
    }

    async fn setup_emergency_logging(&self) {
        #[cfg(target_arch = "wasm32")]
        {
            // Ensure WASM logging works
            if tracing_wasm::try_set_as_global_default().is_err() {
                web_sys::console::error_1(&"Failed to set up tracing-wasm".into());
            }
            web_sys::console::log_1(&"🔧 Emergency logging setup complete".into());
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            // Ensure native logging works
            if tracing_subscriber::fmt()
                .with_max_level(tracing::Level::DEBUG)
                .with_target(false)
                .try_init()
                .is_err()
            {
                eprintln!("Failed to initialize tracing subscriber");
            }
            println!("🔧 Emergency logging setup complete");
        }
    }

    async fn run_init_step(&self, step_name: &str, app: &mut MinimalApp) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        match step_name {
            "logging" => {
                app.setup_logging().await?;
                Ok(())
            }
            "platform" => {
                app.setup_platform().await?;
                Ok(())
            }
            "config" => {
                app.setup_config().await?;
                Ok(())
            }
            "event_bus" => {
                app.setup_event_bus().await?;
                Ok(())
            }
            "plugin_registry" => {
                app.setup_plugin_registry().await?;
                Ok(())
            }
            "ui" => {
                app.setup_ui().await?;
                Ok(())
            }
            _ => Err(format!("Unknown initialization step: {}", step_name).into())
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum InitError {
    #[error("Initialization step '{step}' failed: {error} (took {duration:?})")]
    StepFailed {
        step: String,
        error: String,
        duration: Duration,
    },
    #[error("Initialization step '{step}' timed out after {timeout:?}")]
    Timeout {
        step: String,
        timeout: Duration,
    },
}

/// Minimal application that focuses on getting the basics working
pub struct MinimalApp {
    pub state: AppState,
    pub logging_ready: bool,
    pub platform_ready: bool,
    pub config_ready: bool,
    pub event_bus_ready: bool,
    pub plugin_registry_ready: bool,
    pub ui_ready: bool,
}

#[derive(Debug, Default, Clone)]
pub struct AppState {
    pub user: Option<String>,
    pub theme: String,
    pub error_message: Option<String>,
    pub plugins_loaded: usize,
}

impl MinimalApp {
    pub fn new() -> Self {
        Self {
            state: AppState {
                theme: "default".to_string(),
                ..Default::default()
            },
            logging_ready: false,
            platform_ready: false,
            config_ready: false,
            event_bus_ready: false,
            plugin_registry_ready: false,
            ui_ready: false,
        }
    }

    async fn setup_logging(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        #[cfg(target_arch = "wasm32")]
        {
            web_sys::console::log_1(&"🔧 Setting up WASM logging".into());
            tracing_wasm::set_as_global_default();
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            println!("🔧 Setting up native logging");
            tracing_subscriber::fmt()
                .with_max_level(tracing::Level::DEBUG)
                .with_target(false)
                .init();
        }

        self.logging_ready = true;
        info!("📝 Logging system initialized");
        Ok(())
    }

    async fn setup_platform(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("🖥️  Setting up platform layer");

        #[cfg(target_arch = "wasm32")]
        {
            // Minimal WASM platform setup
            if web_sys::window().is_none() {
                return Err("No window object available".into());
            }
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            // Minimal native platform setup
            let data_dir = std::env::temp_dir().join("qorzen_debug");
            tokio::fs::create_dir_all(&data_dir).await?;
        }

        self.platform_ready = true;
        info!("🖥️  Platform layer ready");
        Ok(())
    }

    async fn setup_config(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("⚙️  Setting up configuration");
        // Minimal config - just set some defaults
        self.config_ready = true;
        info!("⚙️  Configuration ready");
        Ok(())
    }

    async fn setup_event_bus(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("📡 Setting up event bus");
        // Minimal event bus setup
        self.event_bus_ready = true;
        info!("📡 Event bus ready");
        Ok(())
    }

    async fn setup_plugin_registry(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("🧩 Setting up plugin registry");

        // Initialize the plugin factory registry
        crate::plugin::PluginFactoryRegistry::initialize();

        // Try to register minimal plugins
        match crate::plugin::builtin::register_builtin_plugins().await {
            Ok(()) => {
                let plugins = crate::plugin::PluginFactoryRegistry::list_plugins().await;
                self.state.plugins_loaded = plugins.len();
                info!("🧩 Loaded {} plugins", plugins.len());
            }
            Err(e) => {
                warn!("🧩 Plugin registration failed (continuing anyway): {}", e);
                self.state.plugins_loaded = 0;
            }
        }

        self.plugin_registry_ready = true;
        info!("🧩 Plugin registry ready");
        Ok(())
    }

    async fn setup_ui(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("🎨 Setting up UI components");
        self.ui_ready = true;
        info!("🎨 UI components ready");
        Ok(())
    }

    pub fn get_status(&self) -> String {
        format!(
            "Logging: {} | Platform: {} | Config: {} | Events: {} | Plugins: {} ({} loaded) | UI: {}",
            if self.logging_ready { "✅" } else { "❌" },
            if self.platform_ready { "✅" } else { "❌" },
            if self.config_ready { "✅" } else { "❌" },
            if self.event_bus_ready { "✅" } else { "❌" },
            if self.plugin_registry_ready { "✅" } else { "❌" },
            self.state.plugins_loaded,
            if self.ui_ready { "✅" } else { "❌" },
        )
    }
}
