// src/ui/pages/plugins.rs
use std::sync::Arc;
use dioxus::prelude::*;
#[allow(unused_imports)]
use dioxus_router::prelude::*;
use tokio::sync::RwLock;
use crate::plugin::{PluginFactoryRegistry,Platform};
use crate::ui::{pages::{EmptyState,PageWrapper},router::Route,};
use crate::ui::services::plugin_service::{use_plugin_service,PluginService};
use crate::ui::state::{use_app_dispatch, AppAction};

#[derive(Debug,Clone,PartialEq)]
pub struct PluginInfo{
    pub id:String,
    pub name:String,
    pub version:String,
    pub author:String,
    pub description:String,
    pub icon:String,
    pub status:PluginStatus,
    pub installed_at:Option<chrono::DateTime<chrono::Utc>>,
    pub error_message:Option<String>,
    pub source:PluginSource,
    pub has_ui_components:bool,
    pub has_menu_items:bool,
    pub has_settings:bool,
    pub install_path:Option<std::path::PathBuf>,
    pub supported_platforms:Vec<Platform>,
    pub permissions:Vec<String>,
    pub dependencies:Vec<String>,
}

#[derive(Debug,Clone,PartialEq)]
pub enum PluginStatus{
    Available,
    Installing,
    Installed,
    Loading,
    Running,
    Failed,
    Uninstalling,
    Updating,
}

impl std::fmt::Display for PluginStatus{
    fn fmt(&self,f:&mut std::fmt::Formatter<'_>)->std::fmt::Result{
        match self{
            Self::Available=>write!(f,"Available"),
            Self::Installing=>write!(f,"Installing"),
            Self::Installed=>write!(f,"Installed"),
            Self::Loading=>write!(f,"Loading"),
            Self::Running=>write!(f,"Running"),
            Self::Failed=>write!(f,"Failed"),
            Self::Uninstalling=>write!(f,"Uninstalling"),
            Self::Updating=>write!(f,"Updating"),
        }
    }
}

#[derive(Debug,Clone,PartialEq)]
pub enum PluginSource{
    Installed,
    Discovered,
    Registry,
    Builtin,
}

async fn install_plugin(plugin_service:Arc<RwLock<PluginService>>,plugin_id:&str,)->crate::error::Result<String>{
    let service=plugin_service.read().await;
    service.install_plugin(plugin_id).await
}

pub async fn uninstall_plugin(plugin_service:Arc<RwLock<PluginService>>,plugin_id:&str,)->crate::error::Result<()>{
    let service=plugin_service.read().await;
    service.uninstall_plugin(plugin_id).await
}

pub async fn discover_plugins(plugin_service:Arc<RwLock<PluginService>>,)->crate::error::Result<usize>{
    let service=plugin_service.read().await;
    service.discover_plugins().await
}

/// Main plugins management page component
#[component]
pub fn Plugins()->Element{
    let mut active_tab=use_signal(||"installed".to_string());
    let mut search_query=use_signal(String::new);
    let mut loading=use_signal(||false);
    let mut installing_plugins=use_signal(||std::collections::HashSet::<String>::new());
    let mut error_message=use_signal(||None::<String>);
    let mut success_message=use_signal(||None::<String>);
    let dispatch=use_app_dispatch();

    let mut installed_plugins=use_resource(move||async move{
        get_installed_plugins().await
    });

    let mut available_plugins=use_resource({
        let query=search_query();
        move||{
            let query=query.clone();
            async move{
                if query.is_empty(){
                    get_available_plugins().await
                }else{
                    search_plugins(&query).await
                }
            }
        }
    });

    let mut registry_plugins=use_resource(move||async move{
        get_registry_plugins().await
    });

    let handle_refresh=move|_|{
        loading.set(true);
        spawn(async move{
            #[cfg(not(target_arch="wasm32"))]
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            #[cfg(target_arch="wasm32")]
            gloo_timers::future::TimeoutFuture::new(500).await;

            installed_plugins.restart();
            available_plugins.restart();
            registry_plugins.restart();
            loading.set(false);
        });
    };

    let plugin_service=use_plugin_service();
    let plugin_service_install=plugin_service.clone();
    let handle_install=move|plugin_id:String|{
        let plugin_service=plugin_service_install.clone();
        let dispatch=dispatch;
        let mut installing=installing_plugins();
        installing.insert(plugin_id.clone());
        installing_plugins.set(installing);

        spawn(async move{
            match install_plugin(plugin_service.clone(),&plugin_id).await{
                Ok(message)=>{
                    success_message.set(Some(message));
                    error_message.set(None);

                    // Update app state with new plugin
                    let all_plugins=PluginFactoryRegistry::get_all_plugin_info().await;
                    dispatch(AppAction::SetAvailablePlugins(all_plugins));

                    installed_plugins.restart();
                    available_plugins.restart();
                    registry_plugins.restart();
                }
                Err(e)=>{
                    error_message.set(Some(format!("Failed to install {}: {}",plugin_id,e)));
                    success_message.set(None);
                }
            }

            let mut installing=installing_plugins();
            installing.remove(&plugin_id);
            installing_plugins.set(installing);
        });
    };

    let plugin_service_uninstall=plugin_service.clone();
    let handle_uninstall=move|plugin_id:String|{
        let plugin_service=plugin_service_uninstall.clone();
        let dispatch=dispatch;

        spawn(async move{
            match uninstall_plugin(plugin_service.clone(),&plugin_id).await{
                Ok(_)=>{
                    success_message.set(Some(format!("Plugin '{}' uninstalled successfully",plugin_id)));
                    error_message.set(None);

                    // Update app state
                    let all_plugins=PluginFactoryRegistry::get_all_plugin_info().await;
                    dispatch(AppAction::SetAvailablePlugins(all_plugins));

                    installed_plugins.restart();
                    available_plugins.restart();
                }
                Err(e)=>{
                    error_message.set(Some(format!("Failed to uninstall {}: {}",plugin_id,e)));
                    success_message.set(None);
                }
            }
        });
    };

    let plugin_service_discover=plugin_service.clone();
    let handle_discover=move|_|{
        let plugin_service=plugin_service_discover.clone();
        loading.set(true);
        spawn(async move{
            match discover_plugins(plugin_service.clone()).await{
                Ok(count)=>{
                    success_message.set(Some(format!("Discovered {} new plugins",count)));
                    error_message.set(None);
                    available_plugins.restart();
                }
                Err(e)=>{
                    error_message.set(Some(format!("Failed to discover plugins: {}",e)));
                    success_message.set(None);
                }
            }
            loading.set(false);
        });
    };

    let page_actions=rsx!{
        div{class:"flex space-x-3",
            button{
                r#type:"button",
                class:"inline-flex items-center px-4 py-2 border border-gray-300 shadow-sm text-sm font-medium rounded-md text-gray-700 bg-white hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500",
                onclick:handle_refresh,
                disabled:loading(),
                if loading(){"Refreshing..."}else{"Refresh"}
            }
            button{
                r#type:"button",
                class:"inline-flex items-center px-4 py-2 border border-transparent text-sm font-medium rounded-md shadow-sm text-white bg-green-600 hover:bg-green-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-green-500",
                onclick:handle_discover,
                disabled:loading(),
                "Discover Plugins"
            }
        }
    };

    rsx!{
        PageWrapper{
            title:"Plugins".to_string(),
            subtitle:Some("Extend your application with plugins".to_string()),
            actions:Some(page_actions),

            if let Some(error)=error_message(){
                AlertMessage{
                    message:error,
                    alert_type:AlertType::Error,
                    on_dismiss:move|_|error_message.set(None)
                }
            }

            if let Some(success)=success_message(){
                AlertMessage{
                    message:success,
                    alert_type:AlertType::Success,
                    on_dismiss:move|_|success_message.set(None)
                }
            }

            if active_tab()=="available"||active_tab()=="registry"{
                div{class:"mb-6",
                    div{class:"relative",
                        input{
                            r#type:"text",
                            placeholder:"Search plugins...",
                            class:"block w-full pr-12 border-gray-300 rounded-md focus:ring-blue-500 focus:border-blue-500 sm:text-sm",
                            value:"{search_query}",
                            oninput:move|e|{
                                search_query.set(e.value());
                                if active_tab()=="available"{
                                    available_plugins.restart();
                                }else if active_tab()=="registry"{
                                    registry_plugins.restart();
                                }
                            }
                        }
                    }
                }
            }

            div{class:"border-b border-gray-200 mb-6",
                nav{class:"-mb-px flex space-x-8",
                    TabButton{
                        active:active_tab()=="installed",
                        onclick:move|_|active_tab.set("installed".to_string()),
                        text:"Installed",
                        count:if let Some(Ok(plugins))=installed_plugins.read().as_ref(){
                            Some(plugins.len())
                        }else{None}
                    }
                    TabButton{
                        active:active_tab()=="available",
                        onclick:move|_|active_tab.set("available".to_string()),
                        text:"Available",
                        count:if let Some(Ok(plugins))=available_plugins.read().as_ref(){
                            Some(plugins.len())
                        }else{None}
                    }
                    TabButton{
                        active:active_tab()=="registry",
                        onclick:move|_|active_tab.set("registry".to_string()),
                        text:"Registry",
                        count:if let Some(Ok(plugins))=registry_plugins.read().as_ref(){
                            Some(plugins.len())
                        }else{None}
                    }
                }
            }

            match active_tab().as_str(){
                "installed"=>rsx!{
                    InstalledPluginsTab{
                        plugins_resource:installed_plugins,
                        installing_plugins:installing_plugins(),
                        on_uninstall:handle_uninstall
                    }
                },
                "available"=>rsx!{
                    AvailablePluginsTab{
                        plugins_resource:available_plugins,
                        installing_plugins:installing_plugins(),
                        on_install:handle_install
                    }
                },
                "registry"=>rsx!{
                    RegistryPluginsTab{
                        plugins_resource:registry_plugins,
                        installing_plugins:installing_plugins(),
                        on_install:handle_install
                    }
                },
                _=>rsx!{div{"Unknown tab"}}
            }
        }
    }
}

#[derive(Debug,Clone,PartialEq)]
enum AlertType{
    Success,
    Error,
    Warning,
    Info,
}

#[component]
fn AlertMessage(message:String,alert_type:AlertType,on_dismiss:EventHandler<()>)->Element{
    let (bg_color,border_color,text_color,icon)=match alert_type{
        AlertType::Success=>("bg-green-50","border-green-200","text-green-800","✓"),
        AlertType::Error=>("bg-red-50","border-red-200","text-red-800","✗"),
        AlertType::Warning=>("bg-yellow-50","border-yellow-200","text-yellow-800","⚠"),
        AlertType::Info=>("bg-blue-50","border-blue-200","text-blue-800","ℹ"),
    };

    rsx!{
        div{class:"mb-6 {bg_color} border {border_color} rounded-md p-4",
            div{class:"flex",
                div{class:"flex-shrink-0",
                    span{class:"text-xl {text_color}","{icon}"}
                }
                div{class:"ml-3 flex-1",
                    p{class:"text-sm {text_color}","{message}"}
                }
                div{class:"ml-auto pl-3",
                    button{
                        r#type:"button",
                        class:"inline-flex rounded-md {bg_color} p-1.5 {text_color} hover:opacity-75 focus:outline-none focus:ring-2 focus:ring-offset-2",
                        onclick:move|_|on_dismiss.call(()),
                        span{class:"sr-only","Dismiss"}
                        "×"
                    }
                }
            }
        }
    }
}

#[component]
fn TabButton(active:bool,onclick:EventHandler<()>,text:String,count:Option<usize>)->Element{
    let class=if active{
        "py-2 px-1 border-b-2 border-blue-500 text-blue-600 font-medium text-sm"
    }else{
        "py-2 px-1 border-b-2 border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300 font-medium text-sm"
    };

    rsx!{
        button{
            r#type:"button",
            class:"{class}",
            onclick:move|_|onclick.call(()),
            "{text}"
            if let Some(count)=count{
                " ({count})"
            }
        }
    }
}

#[component]
fn InstalledPluginsTab(
    plugins_resource:Resource<Result<Vec<PluginInfo>,String>>,
    installing_plugins:std::collections::HashSet<String>,
    on_uninstall:EventHandler<String>,
)->Element{
    match&*plugins_resource.read_unchecked(){
        Some(Ok(plugins))=>{
            if plugins.is_empty(){
                rsx!{
                    EmptyState{
                        icon:"🧩".to_string(),
                        title:"No plugins installed".to_string(),
                        description:"Install plugins to extend your application functionality".to_string()
                    }
                }
            }else{
                rsx!{
                    div{class:"grid grid-cols-1 gap-6 sm:grid-cols-2 lg:grid-cols-3",
                        for plugin in plugins{
                            PluginCard{
                                key:"{plugin.id}",
                                plugin:plugin.clone(),
                                is_installing:installing_plugins.contains(&plugin.id),
                                show_install_button:false,
                                show_uninstall_button:true,
                                on_install:move|_:String|{},
                                on_uninstall:on_uninstall
                            }
                        }
                    }
                }
            }
        }
        Some(Err(error))=>{
            rsx!{
                div{class:"text-center py-12",
                    div{class:"text-6xl text-red-500 mb-4","⚠️"}
                    h2{class:"text-2xl font-bold text-gray-900 mb-2","Failed to load plugins"}
                    p{class:"text-gray-600 mb-6","{error}"}
                }
            }
        }
        None=>{
            rsx!{
                div{class:"grid grid-cols-1 gap-6 sm:grid-cols-2 lg:grid-cols-3",
                    for _ in 0..3{
                        PluginCardSkeleton{}
                    }
                }
            }
        }
    }
}

#[component]
fn AvailablePluginsTab(
    plugins_resource:Resource<Result<Vec<PluginInfo>,String>>,
    installing_plugins:std::collections::HashSet<String>,
    on_install:EventHandler<String>,
)->Element{
    match&*plugins_resource.read_unchecked(){
        Some(Ok(plugins))=>{
            if plugins.is_empty(){
                rsx!{
                    EmptyState{
                        icon:"🔍".to_string(),
                        title:"No plugins available".to_string(),
                        description:"No plugins found in the plugins directory. Add plugins to the 'plugins' folder and click 'Discover'.".to_string()
                    }
                }
            }else{
                rsx!{
                    div{class:"grid grid-cols-1 gap-6 sm:grid-cols-2 lg:grid-cols-3",
                        for plugin in plugins{
                            PluginCard{
                                key:"{plugin.id}",
                                plugin:plugin.clone(),
                                is_installing:installing_plugins.contains(&plugin.id),
                                show_install_button:true,
                                show_uninstall_button:false,
                                on_install:on_install,
                                on_uninstall:move|_:String|{}
                            }
                        }
                    }
                }
            }
        }
        Some(Err(error))=>{
            rsx!{
                div{class:"text-center py-12",
                    div{class:"text-6xl text-red-500 mb-4","⚠️"}
                    h2{class:"text-2xl font-bold text-gray-900 mb-2","Failed to discover plugins"}
                    p{class:"text-gray-600 mb-6","{error}"}
                    p{class:"text-sm text-gray-500","Make sure the 'plugins' directory exists and contains valid plugin manifests."}
                }
            }
        }
        None=>{
            rsx!{
                div{class:"text-center py-12",
                    div{class:"animate-spin rounded-full h-32 w-32 border-b-2 border-blue-600 mx-auto"}
                    p{class:"mt-4 text-gray-600","Discovering plugins..."}
                }
            }
        }
    }
}

#[component]
fn RegistryPluginsTab(
    plugins_resource:Resource<Result<Vec<PluginInfo>,String>>,
    installing_plugins:std::collections::HashSet<String>,
    on_install:EventHandler<String>,
)->Element{
    match&*plugins_resource.read_unchecked(){
        Some(Ok(plugins))=>{
            if plugins.is_empty(){
                rsx!{
                    EmptyState{
                        icon:"🌐".to_string(),
                        title:"No plugins in registry".to_string(),
                        description:"No plugins found in the registry or registry is unavailable.".to_string()
                    }
                }
            }else{
                rsx!{
                    div{class:"grid grid-cols-1 gap-6 sm:grid-cols-2 lg:grid-cols-3",
                        for plugin in plugins{
                            PluginCard{
                                key:"{plugin.id}",
                                plugin:plugin.clone(),
                                is_installing:installing_plugins.contains(&plugin.id),
                                show_install_button:true,
                                show_uninstall_button:false,
                                on_install:on_install,
                                on_uninstall:move|_:String|{}
                            }
                        }
                    }
                }
            }
        }
        Some(Err(error))=>{
            rsx!{
                div{class:"text-center py-12",
                    div{class:"text-6xl text-orange-500 mb-4","🌐"}
                    h2{class:"text-2xl font-bold text-gray-900 mb-2","Registry unavailable"}
                    p{class:"text-gray-600 mb-6","{error}"}
                    p{class:"text-sm text-gray-500","Check your internet connection or try again later."}
                }
            }
        }
        None=>{
            rsx!{
                div{class:"text-center py-12",
                    div{class:"animate-spin rounded-full h-32 w-32 border-b-2 border-blue-600 mx-auto"}
                    p{class:"mt-4 text-gray-600","Loading registry..."}
                }
            }
        }
    }
}

#[component]
fn PluginCard(
    plugin:PluginInfo,
    is_installing:bool,
    show_install_button:bool,
    show_uninstall_button:bool,
    on_install:EventHandler<String>,
    on_uninstall:EventHandler<String>,
)->Element{
    let status_color=match plugin.status{
        PluginStatus::Running=>"bg-green-100 text-green-800",
        PluginStatus::Installed=>"bg-blue-100 text-blue-800",
        PluginStatus::Available=>"bg-gray-100 text-gray-800",
        PluginStatus::Failed=>"bg-red-100 text-red-800",
        PluginStatus::Installing|PluginStatus::Loading=>"bg-yellow-100 text-yellow-800",
        PluginStatus::Uninstalling=>"bg-orange-100 text-orange-800",
        PluginStatus::Updating=>"bg-purple-100 text-purple-800",
    };

    // Check if plugin is actually installed/running
    let is_plugin_available=plugin.status==PluginStatus::Running||plugin.status==PluginStatus::Installed;

    rsx!{
        div{class:"bg-white overflow-hidden shadow rounded-lg hover:shadow-md transition-shadow",
            div{class:"p-6",
                div{class:"flex items-center justify-between",
                    div{class:"flex items-center space-x-3",
                        div{class:"flex-shrink-0",
                            span{class:"text-3xl","{plugin.icon}"}
                        }
                        div{class:"ml-4",
                            h3{class:"text-lg font-medium text-gray-900","{plugin.name}"}
                            p{class:"text-sm text-gray-500","v{plugin.version} by {plugin.author}"}
                        }
                    }
                    div{class:"flex-shrink-0",
                        span{class:"inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium {status_color}",
                            "{plugin.status}"
                        }
                    }
                }

                div{class:"mt-4",
                    p{class:"text-sm text-gray-600","{plugin.description}"}
                }

                if let Some(error)=&plugin.error_message{
                    div{class:"mt-3 text-sm text-red-600",
                        "Error: {error}"
                    }
                }

                div{class:"mt-4 flex flex-wrap gap-2",
                    if plugin.has_ui_components{
                        CapabilityBadge{icon:"🖼️",text:"UI Components"}
                    }
                    if plugin.has_menu_items{
                        CapabilityBadge{icon:"📋",text:"Menu Items"}
                    }
                    if plugin.has_settings{
                        CapabilityBadge{icon:"⚙️",text:"Configurable"}
                    }
                    if!plugin.permissions.is_empty(){
                        CapabilityBadge{icon:"🔐",text:format!("{} permissions",plugin.permissions.len())}
                    }
                    if!plugin.dependencies.is_empty(){
                        CapabilityBadge{icon:"🔗",text:format!("{} dependencies",plugin.dependencies.len())}
                    }
                }

                div{class:"mt-4 flex items-center text-xs text-gray-500 space-x-4",
                    div{class:"flex items-center",
                        span{class:"mr-1","📁"}
                        match plugin.source{
                            PluginSource::Builtin=>"Built-in",
                            PluginSource::Discovered=>"Discovered",
                            PluginSource::Installed=>"Installed",
                            PluginSource::Registry=>"Registry"
                        }
                    }
                    if!plugin.supported_platforms.is_empty(){
                        div{class:"flex items-center",
                            span{class:"mr-1","💻"}
                            "{plugin.supported_platforms.len()} platforms"
                        }
                    }
                }
            }

            div{class:"mt-6 flex space-x-3",
                match plugin.source{
                    PluginSource::Installed|PluginSource::Builtin=>{
                        if is_plugin_available{
                            rsx!{
                                Link{
                                    to:Route::Plugin{plugin_id:plugin.id.clone()},
                                    class:"flex-1 bg-white py-2 px-3 border border-gray-300 rounded-md shadow-sm text-sm leading-4 font-medium text-gray-700 hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 text-center",
                                    "Open Plugin"
                                }
                                Link{
                                    to:Route::PluginSettings{plugin_id:plugin.id.clone()},
                                    class:"bg-blue-600 py-2 px-3 border border-transparent rounded-md shadow-sm text-sm leading-4 font-medium text-white hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500",
                                    "Settings"
                                }
                                if show_uninstall_button&&plugin.source!=PluginSource::Builtin{
                                    button{
                                        r#type:"button",
                                        class:"bg-red-600 py-2 px-3 border border-transparent rounded-md shadow-sm text-sm leading-4 font-medium text-white hover:bg-red-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-red-500",
                                        onclick:move|_|on_uninstall.call(plugin.id.clone()),
                                        "Uninstall"
                                    }
                                }
                            }
                        }else{
                            rsx!{
                                button{
                                    r#type:"button",
                                    class:"flex-1 bg-blue-600 py-2 px-3 border border-transparent rounded-md shadow-sm text-sm leading-4 font-medium text-white hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500",
                                    onclick:move|_|on_install.call(plugin.id.clone()),
                                    "Load Plugin"
                                }
                            }
                        }
                    },
                    _=>{
                        if show_install_button{
                            rsx!{
                                button{
                                    r#type:"button",
                                    class:"flex-1 bg-blue-600 py-2 px-3 border border-transparent rounded-md shadow-sm text-sm leading-4 font-medium text-white hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 disabled:opacity-50",
                                    disabled:is_installing,
                                    onclick:move|_|on_install.call(plugin.id.clone()),
                                    if is_installing{"Installing..."}else{"Install"}
                                }
                                button{
                                    r#type:"button",
                                    class:"bg-white py-2 px-3 border border-gray-300 rounded-md shadow-sm text-sm leading-4 font-medium text-gray-700 hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500",
                                    "Details"
                                }
                            }
                        }else{
                            rsx!{
                                button{
                                    r#type:"button",
                                    class:"flex-1 bg-white py-2 px-3 border border-gray-300 rounded-md shadow-sm text-sm leading-4 font-medium text-gray-700 hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500",
                                    "View Details"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn CapabilityBadge(icon:String,text:String)->Element{
    rsx!{
        span{class:"inline-flex items-center px-2 py-1 rounded-md text-xs font-medium bg-gray-100 text-gray-800",
            span{class:"mr-1","{icon}"}
            "{text}"
        }
    }
}

#[component]
fn PluginCardSkeleton()->Element{
    rsx!{
        div{class:"animate-pulse",
            div{class:"bg-white overflow-hidden shadow rounded-lg",
                div{class:"p-6",
                    div{class:"flex items-center justify-between mb-4",
                        div{class:"flex items-center space-x-3",
                            div{class:"w-12 h-12 bg-gray-200 rounded-lg"}
                            div{
                                div{class:"h-4 bg-gray-200 rounded w-24 mb-2"}
                                div{class:"h-3 bg-gray-200 rounded w-16"}
                            }
                        }
                        div{class:"h-6 bg-gray-200 rounded w-16"}
                    }
                    div{class:"h-3 bg-gray-200 rounded mb-2"}
                    div{class:"h-3 bg-gray-200 rounded w-3/4"}
                }
            }
        }
    }
}

/// Plugin detail view component
#[component]
pub fn PluginView(plugin_id:String,page:Option<String>)->Element{
    let plugin_info=use_resource({
        let plugin_id=plugin_id.clone();
        move||{
            let plugin_id=plugin_id.clone();
            async move{
                PluginFactoryRegistry::get_plugin_info(&plugin_id).await
            }
        }
    });

    rsx!{
        PageWrapper{
            title:format!("Plugin: {}",plugin_id),
            subtitle:Some("Plugin configuration and management".to_string()),

            match&*plugin_info.read_unchecked(){
                Some(Some(info))=>rsx!{
                    div{class:"space-y-6",
                        div{class:"bg-white shadow rounded-lg p-6",
                            div{class:"flex items-center mb-6",
                                span{class:"text-4xl mr-4","{get_plugin_icon(&info.id)}"}
                                div{
                                    h2{class:"text-2xl font-semibold text-gray-900","{info.name}"}
                                    p{class:"text-gray-600","v{info.version} by {info.author}"}
                                    p{class:"text-sm text-gray-500 mt-1","{info.description}"}
                                }
                            }

                            if let Some(page_name)=page{
                                div{class:"mb-4 p-3 bg-blue-50 rounded-md",
                                    p{class:"text-sm text-blue-800","Settings for {page_name}"}
                                }
                            }

                            div{class:"grid grid-cols-1 md:grid-cols-2 gap-6",
                                div{class:"p-4 bg-gray-50 rounded-lg",
                                    h3{class:"font-medium text-gray-900 mb-2","Plugin Information"}
                                    dl{class:"space-y-2 text-sm",
                                        InfoRow{label:"ID",value:info.id.clone()}
                                        InfoRow{label:"Version",value:info.version.clone()}
                                        InfoRow{label:"Author",value:info.author.clone()}
                                        InfoRow{label:"License",value:info.license.clone()}
                                        InfoRow{label:"Min Core Version",value:info.minimum_core_version.clone()}
                                        if let Some(ref homepage)=info.homepage{
                                            InfoRow{label:"Homepage",value:homepage.clone()}
                                        }
                                        if let Some(ref repository)=info.repository{
                                            InfoRow{label:"Repository",value:repository.clone()}
                                        }
                                    }
                                }
                                div{class:"p-4 bg-green-50 rounded-lg",
                                    h3{class:"font-medium text-gray-900 mb-2","Status"}
                                    div{class:"flex items-center",
                                        div{class:"w-3 h-3 bg-green-500 rounded-full mr-2"}
                                        span{class:"text-sm text-green-700","Plugin is loaded and active"}
                                    }
                                }
                            }

                            div{class:"mt-6",
                                h3{class:"font-medium text-gray-900 mb-2","Supported Platforms"}
                                div{class:"flex flex-wrap gap-2",
                                    for platform in&info.supported_platforms{
                                        span{class:"inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-blue-100 text-blue-800",
                                            "{platform:?}"
                                        }
                                    }
                                }
                            }

                            div{class:"mt-6 p-4 bg-blue-50 rounded-md",
                                p{class:"text-sm text-blue-800","🔌 Plugin settings interface would be rendered here."}
                            }
                        }
                    }
                },
                Some(None)=>rsx!{
                    div{class:"bg-white shadow rounded-lg p-6",
                        div{class:"text-center py-12",
                            div{class:"text-6xl text-gray-400 mb-4","🧩"}
                            h2{class:"text-2xl font-bold text-gray-900 mb-2","Plugin Not Found"}
                            p{class:"text-gray-600","The plugin '{plugin_id}' could not be found."}
                        }
                    }
                },
                None=>rsx!{
                    div{class:"bg-white shadow rounded-lg p-6",
                        div{class:"animate-pulse",
                            div{class:"flex items-center mb-6",
                                div{class:"w-16 h-16 bg-gray-200 rounded mr-4"}
                                div{
                                    div{class:"h-6 bg-gray-200 rounded w-48 mb-2"}
                                    div{class:"h-4 bg-gray-200 rounded w-32"}
                                }
                            }
                            div{class:"h-4 bg-gray-200 rounded mb-2"}
                            div{class:"h-4 bg-gray-200 rounded w-3/4"}
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn InfoRow(label:String,value:String)->Element{
    rsx!{
        div{class:"flex justify-between",
            dt{class:"text-gray-500","{label}:"}
            dd{class:"text-gray-900","{value}"}
        }
    }
}

async fn get_installed_plugins()->Result<Vec<PluginInfo>,String>{
    let plugin_service=use_plugin_service();
    let service=plugin_service.read().await;
    let all_plugins=PluginFactoryRegistry::get_all_plugin_info().await;
    let mut installed_plugins=Vec::new();

    for plugin_info in all_plugins{
        let status=service.get_plugin_status(&plugin_info.id).await
            .ok()
            .unwrap_or_else(||"unknown".to_string());

        let plugin_status=match status.as_str(){
            "loaded"=>PluginStatus::Running,
            "available"=>PluginStatus::Installed,
            _=>PluginStatus::Available,
        };

        // Only include plugins that are actually installed/loaded
        if matches!(plugin_status,PluginStatus::Running|PluginStatus::Installed){
            installed_plugins.push(PluginInfo{
                id:plugin_info.id.clone(),
                name:plugin_info.name.clone(),
                version:plugin_info.version.clone(),
                author:plugin_info.author.clone(),
                description:plugin_info.description.clone(),
                icon:get_plugin_icon(&plugin_info.id),
                status:plugin_status,
                installed_at:Some(chrono::Utc::now()),
                error_message:None,
                source:PluginSource::Builtin,
                has_ui_components:true,
                has_menu_items:true,
                has_settings:true,
                install_path:None,
                supported_platforms:plugin_info.supported_platforms,
                permissions:vec!["ui.render".to_string(),"data.read".to_string()],
                dependencies:vec![],
            });
        }
    }

    Ok(installed_plugins)
}

async fn get_available_plugins()->Result<Vec<PluginInfo>,String>{
    let discovered=discover_plugins_from_directory().await?;
    let installed_ids:std::collections::HashSet<String>=PluginFactoryRegistry::list_plugins().await
        .into_iter()
        .collect();

    let available=discovered.into_iter()
        .filter(|plugin|!installed_ids.contains(&plugin.id))
        .collect();

    Ok(available)
}

async fn get_registry_plugins()->Result<Vec<PluginInfo>,String>{
    #[cfg(not(target_arch="wasm32"))]
    tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
    #[cfg(target_arch="wasm32")]
    gloo_timers::future::TimeoutFuture::new(1000).await;

    Ok(vec![
        PluginInfo{
            id:"advanced_analytics".to_string(),
            name:"Advanced Analytics".to_string(),
            version:"2.1.0".to_string(),
            author:"Analytics Co".to_string(),
            description:"Advanced analytics and reporting capabilities".to_string(),
            icon:"📊".to_string(),
            status:PluginStatus::Available,
            installed_at:None,
            error_message:None,
            source:PluginSource::Registry,
            has_ui_components:true,
            has_menu_items:true,
            has_settings:true,
            install_path:None,
            supported_platforms:vec![Platform::All],
            permissions:vec!["data.read".to_string(),"analytics.generate".to_string()],
            dependencies:vec!["database".to_string()],
        },
        PluginInfo{
            id:"backup_manager".to_string(),
            name:"Backup Manager".to_string(),
            version:"1.5.3".to_string(),
            author:"Backup Solutions".to_string(),
            description:"Automated backup and restore functionality".to_string(),
            icon:"💾".to_string(),
            status:PluginStatus::Available,
            installed_at:None,
            error_message:None,
            source:PluginSource::Registry,
            has_ui_components:true,
            has_menu_items:true,
            has_settings:true,
            install_path:None,
            supported_platforms:vec![Platform::Windows,Platform::MacOS,Platform::Linux],
            permissions:vec!["filesystem.write".to_string(),"system.backup".to_string()],
            dependencies:vec![],
        },
    ])
}

async fn discover_plugins_from_directory()->Result<Vec<PluginInfo>,String>{
    let plugins_dir=std::path::PathBuf::from("plugins");

    #[cfg(not(target_arch="wasm32"))]
    {
        if!plugins_dir.exists(){
            return Ok(vec![]);
        }

        let mut discovered=Vec::new();
        let mut dir_entries=tokio::fs::read_dir(&plugins_dir).await
            .map_err(|e|format!("Failed to read plugins directory: {}",e))?;

        while let Some(entry)=dir_entries.next_entry().await
            .map_err(|e|format!("Failed to read directory entry: {}",e))?
        {
            if entry.file_type().await
                .map_err(|e|format!("Failed to get file type: {}",e))?
                .is_dir()
            {
                let plugin_dir=entry.path();
                let manifest_path=plugin_dir.join("plugin.toml");

                if manifest_path.exists(){
                    match load_plugin_manifest(&manifest_path).await{
                        Ok(plugin_info)=>{
                            discovered.push(plugin_info);
                        }
                        Err(e)=>{
                            tracing::warn!("Failed to load plugin manifest at {:?}: {}",manifest_path,e);
                        }
                    }
                }
            }
        }

        Ok(discovered)
    }

    #[cfg(target_arch="wasm32")]
    {
        Ok(vec![])
    }
}

async fn load_plugin_manifest(manifest_path:&std::path::Path)->Result<PluginInfo,String>{
    #[cfg(not(target_arch="wasm32"))]
    {
        let content=tokio::fs::read_to_string(manifest_path).await
            .map_err(|e|format!("Failed to read manifest file: {}",e))?;

        let manifest:toml::Value=toml::from_str(&content)
            .map_err(|e|format!("Failed to parse TOML: {}",e))?;

        let plugin_section=manifest.get("plugin")
            .ok_or("No [plugin] section found in manifest")?;

        let id=plugin_section.get("id")
            .and_then(|v|v.as_str())
            .ok_or("Plugin ID not found")?
            .to_string();

        let name=plugin_section.get("name")
            .and_then(|v|v.as_str())
            .ok_or("Plugin name not found")?
            .to_string();

        let version=plugin_section.get("version")
            .and_then(|v|v.as_str())
            .ok_or("Plugin version not found")?
            .to_string();

        let author=plugin_section.get("author")
            .and_then(|v|v.as_str())
            .unwrap_or("Unknown")
            .to_string();

        let description=plugin_section.get("description")
            .and_then(|v|v.as_str())
            .unwrap_or("No description")
            .to_string();

        Ok(PluginInfo{
            id:id.clone(),
            name,
            version,
            author,
            description,
            icon:get_plugin_icon(&id),
            status:PluginStatus::Available,
            installed_at:None,
            error_message:None,
            source:PluginSource::Discovered,
            has_ui_components:true,
            has_menu_items:true,
            has_settings:true,
            install_path:Some(manifest_path.parent().unwrap().to_path_buf()),
            supported_platforms:vec![Platform::All],
            permissions:vec!["ui.render".to_string()],
            dependencies:vec![],
        })
    }

    #[cfg(target_arch="wasm32")]
    {
        Err("File system access not available in WASM".to_string())
    }
}

async fn search_plugins(query:&str)->Result<Vec<PluginInfo>,String>{
    let all_available=get_available_plugins().await?;
    Ok(all_available.into_iter()
        .filter(|p|{
            p.name.to_lowercase().contains(&query.to_lowercase())||
                p.description.to_lowercase().contains(&query.to_lowercase())
        })
        .collect())
}

fn get_plugin_icon(plugin_id:&str)->String{
    match plugin_id{
        "system_monitor"=>"🖥️".to_string(),
        "notifications"=>"🔔".to_string(),
        "product_catalog"=>"📦".to_string(),
        "advanced_analytics"=>"📊".to_string(),
        "backup_manager"=>"💾".to_string(),
        _=>"🧩".to_string(),
    }
}

#[cfg(test)]
mod tests{
    use super::*;

    #[test]
    fn test_plugin_status_display(){
        assert_eq!(PluginStatus::Available.to_string(),"Available");
        assert_eq!(PluginStatus::Running.to_string(),"Running");
        assert_eq!(PluginStatus::Failed.to_string(),"Failed");
    }

    #[test]
    fn test_plugin_icon(){
        assert_eq!(get_plugin_icon("system_monitor"),"🖥️");
        assert_eq!(get_plugin_icon("unknown"),"🧩");
    }

    #[tokio::test]
    async fn test_get_registry_plugins(){
        let plugins=get_registry_plugins().await.unwrap();
        assert!(!plugins.is_empty());
        assert!(plugins.iter().any(|p|p.id=="advanced_analytics"));
    }
}