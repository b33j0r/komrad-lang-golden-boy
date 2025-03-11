use crate::execute::Execute;
use crate::module::ModuleCommand;
use crate::module::ModuleId;
use crate::scope::Scope;
use crate::try_bind::TryBind;
use komrad_ast::prelude::{ChannelListener, Message, Value};
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

pub struct ModuleActor {
    pub id: ModuleId,
    pub name: String,
    pub command_rx: mpsc::Receiver<ModuleCommand>,
    pub(crate) scope: Scope,
    pub(crate) channel_listener: ChannelListener,
}

impl ModuleActor {
    pub async fn run(mut self) {
        loop {
            tokio::select! {
                maybe_msg = self.channel_listener.recv() => {
                    match maybe_msg {
                        Ok(message) => {
                            info!("Module {} received message: {:?}", self.name, message);

                            // 2) Dispatch the message by matching all handlers
                            let result = self.dispatch_message(message).await;
                            if let Value::Error(e) = result {
                                warn!("Failed to handle message in Module {}: {}", self.name, e);
                            }
                        },
                        Err(e) => {
                            warn!("Module {} failed to receive message: {}", self.name, e);
                            break;
                        }
                    }
                },
                command = self.command_rx.recv() => {
                    match command {
                        Some(command) => {
                            info!("Module {} received command: {:?}", self.name, command);
                            match command {
                                ModuleCommand::Stop => {
                                    info!("Module {} received Stop command", self.name);
                                    break;
                                }
                                ModuleCommand::Send(message) => {
                                    // Handle sending a message.
                                    info!("Module {} received message: {:?}", self.name, message);
                                }
                                ModuleCommand::ExecuteStatement(statement) => {
                                    // Handle executing a statement.
                                    info!("Module {} executing statement: {:?}", self.name, statement);
                                    // Execute the statement in the module's scope.
                                    if let Value::Error(e) = statement.execute(&mut self.scope).await {
                                        warn!("Failed to execute statement in Module {}: {}", self.name, e);
                                    }
                                }
                                ModuleCommand::ExecuteStatements(statements) => {
                                    // Handle executing multiple statements.
                                    for statement in statements {
                                        info!("Module {} executing statement: {:?}", self.name, statement);
                                        if let Value::Error(e) = statement.execute(&mut self.scope).await {
                                            warn!("Failed to execute statement in Module {}: {}", self.name, e);
                                        }
                                    }
                                }
                                ModuleCommand::QueryScope(sender) => {
                                    // Send the current scope back to the requester.
                                    if let Err(_) = sender.send(self.scope.clone()) {
                                        warn!("Failed to send scope back to requester");
                                    }
                                }
                                ModuleCommand::ModifyScope { key, value } => {
                                    // Modify the module's scope.
                                    self.scope.set(key.clone(), value.clone()).await;
                                    info!("Module {} modified scope: {} = {:?}", self.name, key, value);
                                }
                            }
                        },
                        None => break, // Command channel closed unexpectedly.
                    }
                },
            }
        }
    }

    /// Dispatches a single incoming message by matching all handlers in scope.
    /// Returns whatever the matched handler’s block evaluates to, or Value::Empty if no match.
    async fn dispatch_message(&mut self, message: Message) -> Value {
        // 1) Grab the list of handlers from the scope
        let handlers = self.scope.get_handlers().await;

        // 2) Try each handler’s pattern
        for handler in handlers {
            debug!("Checking handler: {:?}", handler);
            let pattern = handler.pattern();
            if let Some(mut scope) = pattern.try_bind(message.clone(), &mut self.scope).await {
                // 3) If the pattern matches, execute the block
                let block = handler.block();
                let result = block.execute(&mut scope).await;
                debug!("Handler executed successfully, result: {:?}", result);
                return result;
            } else {
                debug!("Pattern did not match: {:?}", pattern);
            }
        }

        warn!("No handler matched for message: {:?}", message.terms());
        Value::Empty
    }
}
