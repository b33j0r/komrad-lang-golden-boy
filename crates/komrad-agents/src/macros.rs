#[macro_export]
macro_rules! stateless_agent_impl {
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

        #[async_trait::async_trait]
        impl komrad_agent::AgentLifecycle for $agent_name {
            async fn get_scope(
                &self,
            ) -> std::sync::Arc<tokio::sync::Mutex<komrad_agent::scope::Scope>> {
                // An empty scope
                std::sync::Arc::new(tokio::sync::Mutex::new(komrad_agent::scope::Scope::new()))
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
macro_rules! stateful_agent_impl {
    ($agent_name:ident) => {
        #[async_trait::async_trait]
        impl komrad_agent::AgentLifecycle for $agent_name {
            async fn get_scope(
                &self,
            ) -> std::sync::Arc<tokio::sync::Mutex<komrad_agent::scope::Scope>> {
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
