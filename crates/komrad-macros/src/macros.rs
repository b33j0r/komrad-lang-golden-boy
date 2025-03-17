#[macro_export]
macro_rules! agent_lifecycle_impl {
    ($agent_name:ident) => {
        #[async_trait::async_trait]
        impl komrad_ast::prelude::AgentLifecycle for $agent_name {
            async fn get_scope(
                &self,
            ) -> std::sync::Arc<tokio::sync::Mutex<komrad_ast::prelude::Scope>> {
                Arc::new(tokio::sync::Mutex::new(komrad_ast::prelude::Scope::new()))
            }
            fn channel(&self) -> &komrad_ast::prelude::Channel {
                &self.channel
            }
            fn listener(&self) -> Arc<komrad_ast::prelude::ChannelListener> {
                self.listener.clone()
            }
        }
    };
}

#[macro_export]
macro_rules! agent_stateless_impl {
    ($agent_name:ident) => {
        impl $agent_name {
            pub fn new() -> Arc<Self> {
                let (channel, listener) = komrad_ast::prelude::Channel::new(32);
                Arc::new(Self {
                    channel,
                    listener: Arc::new(listener),
                })
            }
        }

        komrad_macros::agent_lifecycle_impl!($agent_name);
    };
}

#[macro_export]
macro_rules! agent_stateful_impl {
    ($agent_name:ident) => {
        #[async_trait::async_trait]
        impl komrad_ast::prelude::AgentLifecycle for $agent_name {
            async fn get_scope(
                &self,
            ) -> std::sync::Arc<tokio::sync::Mutex<komrad_ast::prelude::Scope>> {
                self.scope.clone()
            }
            fn channel(&self) -> &komrad_ast::prelude::Channel {
                &self.channel
            }
            fn listener(&self) -> Arc<komrad_ast::prelude::ChannelListener> {
                self.listener.clone()
            }
        }
    };
}
