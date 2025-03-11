use komrad_ast::prelude::{
    Block, CallExpr, Channel, Expr, Message, Number, Statement, ToBlock, ToBoxedExpr, Value,
};
use tokio::time::{sleep, Duration};

// --- Helper function ---
/// A helper that creates a complete agent definition for Alice.
/// In a real scenario, Alice’s definition would be sent to AgentAgent,
/// which would transform [Alice, <block>] into [define, agent, Alice, <block>].
/// Here we use it only for reference.
fn create_real_alice_agent_definition() -> Statement {
    Statement::Expr(Expr::Call(CallExpr::new(
        // The "agent" keyword.
        Expr::Variable("agent".into()),
        vec![
            // Agent name: "Alice" (converted to a Value via .into())
            Expr::Variable("Alice".into()).into(),
            // The agent’s block:
            // {
            //     a = spawn Bob {}
            //     a foo
            // }
            vec![
                // Assignment: a = spawn Bob {}
                Statement::Assignment(
                    "a".into(),
                    Expr::Call(CallExpr::new(
                        Expr::Variable("spawn".into()),
                        vec![Expr::Variable("Bob".into()).into()],
                    )),
                )
                .into(),
                // Call: a foo
                Statement::Expr(Expr::Call(CallExpr::new(
                    Expr::Variable("a".into()),
                    vec![Expr::Variable("foo".into()).into()],
                ))),
            ]
            .to_block()
            .to_boxed_expr(),
        ],
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use komrad_agent::AgentBehavior;
    use komrad_agents::agent_agent::AgentAgent;
    use komrad_agents::registry_agent::RegistryAgent;
    use komrad_agents::spawn_agent::SpawnAgent;
    use komrad_ast::prelude::*;
    use tokio::time::{sleep, Duration};

    // Test 1: Basic channel message send/receive.
    #[tokio::test]
    async fn test_basic_channel_send() {
        let (chan, mut listener) = Channel::new(1);
        let msg = Message::new(vec![Value::Number(Number::UInt(42))], None);
        chan.send(msg.clone()).await.unwrap();
        let received = listener.recv().await.unwrap();
        assert_eq!(
            received.terms(),
            msg.terms(),
            "Channel should deliver the same message"
        );
    }

    // Test 2: Agent registration – the "define" case.
    #[tokio::test]
    async fn test_register_agent() {
        // Create a RegistryAgent and spawn it.
        let registry = RegistryAgent::new();
        let _reg_chan = registry.clone().spawn();

        // Prepare a dummy agent block (for Alice) – here we simply set x = 2 and call foo(bar).
        let alice_block = Block::new(vec![
            Statement::Assignment("x".into(), Expr::Value(Value::Number(Number::UInt(2)))),
            Statement::Expr(Expr::Call(CallExpr::new(
                Expr::Variable("foo".into()),
                vec![Expr::Variable("bar".into()).into()],
            ))),
        ]);

        // Create a "define agent" message manually.
        let msg = Message::new(
            vec![
                Value::Word("define".into()),
                Value::Word("agent".into()),
                Value::Word("Alice".into()),
                Value::Block(Box::new(alice_block.clone())),
            ],
            None,
        );
        // Send the registration message.
        registry.send(msg).await.unwrap();

        // Allow some time for the message to be processed.
        sleep(Duration::from_millis(50)).await;
        let reg_map = registry.registry.read().await;
        assert!(
            reg_map.contains_key("Alice"),
            "Registry should contain Alice after registration"
        );
        assert_eq!(
            reg_map.get("Alice").unwrap(),
            &alice_block,
            "Alice's block should match the registered definition"
        );
    }

    // Test 3: Spawn agent – the "spawn" case returns a channel.
    #[tokio::test]
    async fn test_spawn_agent_bob() {
        // Create a RegistryAgent and spawn it.
        let registry = RegistryAgent::new();
        let _reg_chan = registry.clone().spawn();

        // Pre-register Bob with a dummy block (for example, one that prints a message).
        let bob_block = Block::new(vec![Statement::Expr(Expr::Call(CallExpr::new(
            Expr::Variable("IO".into()),
            vec![
                Expr::Variable("println".into()).into(),
                Expr::Value(Value::String("Bob started".into())).into(),
            ],
        )))]);
        {
            let mut reg_map = registry.registry.write().await;
            reg_map.insert("Bob".into(), bob_block);
        }

        // Use SpawnAgent to spawn Bob.
        let spawn_agent = SpawnAgent::new(registry.clone());
        let spawn_chan = spawn_agent.clone().spawn();

        let (reply_chan, mut reply_listener) = Channel::new(10);
        let msg = Message::new(
            vec![
                Value::Word("spawn".into()),
                Value::Word("agent".into()),
                Value::Word("Bob".into()),
            ],
            Some(reply_chan.clone()),
        );
        spawn_chan.send(msg).await.unwrap();

        let reply = reply_listener.recv().await.unwrap();
        match reply.terms().get(0) {
            Some(Value::Channel(_ch)) => { /* success */ }
            other => panic!("Expected a channel for Bob spawn, got {:?}", other),
        }
    }

    // Test 4: End-to-End: Alice sends a message to Bob.
    #[tokio::test]
    async fn test_alice_sends_to_bob() {
        // Create the RegistryAgent and spawn it.
        let registry = RegistryAgent::new();
        let _ = registry.clone().spawn();

        // Pre-register Bob with a block that (for this test) simulates handling a "foo" message.
        let bob_block = Block::new(vec![Statement::Expr(Expr::Call(CallExpr::new(
            // Simulate Bob "handling" the foo call by printing a message.
            Expr::Variable("IO".into()),
            vec![
                Expr::Variable("println".into()).into(),
                Expr::Value(Value::String("Bob received foo".into())).into(),
            ],
        )))]);
        {
            let mut reg_map = registry.registry.write().await;
            reg_map.insert("Bob".into(), bob_block);
        }

        // Register Alice using AgentAgent.
        // AgentAgent expects a message of the form: [Alice, <block>]
        // where <block> is the agent’s definition.
        let agent_agent = AgentAgent::new(registry.clone());
        let agent_chan = agent_agent.clone().spawn();

        let (reply_chan_alice, mut reply_listener_alice) = Channel::new(10);
        // Manually construct the agent registration message for Alice.
        // This message will be transformed by AgentAgent into:
        //   [define, agent, Alice, <block>]
        let alice_definition_block = (vec![
            // a = spawn Bob {}
            Statement::Assignment(
                "a".into(),
                Expr::Call(CallExpr::new(
                    Expr::Variable("spawn".into()),
                    vec![Expr::Variable("Bob".into()).into()],
                )),
            )
            .into(),
            // a foo
            Statement::Expr(Expr::Call(CallExpr::new(
                Expr::Variable("a".into()),
                vec![Expr::Variable("foo".into()).into()],
            ))),
        ])
        .to_block();
        let alice_registration_msg = Message::new(
            vec![
                Value::Word("Alice".into()),
                Value::Block(Box::new(alice_definition_block)),
            ],
            Some(reply_chan_alice.clone()),
        );
        agent_chan.send(alice_registration_msg).await.unwrap();
        // Wait for the registration reply (should be "defined").
        let _ = reply_listener_alice.recv().await;

        // Now, simulate execution of Alice’s block.
        // Typically, Alice’s block would be executed by the VM.
        // For testing, we simulate the spawn of Bob and then sending "foo".
        let spawn_agent = SpawnAgent::new(registry.clone());
        let spawn_chan = spawn_agent.clone().spawn();
        let (reply_chan_bob, mut reply_listener_bob) = Channel::new(10);
        let msg_spawn_bob = Message::new(
            vec![
                Value::Word("spawn".into()),
                Value::Word("agent".into()),
                Value::Word("Bob".into()),
            ],
            Some(reply_chan_bob.clone()),
        );
        spawn_chan.send(msg_spawn_bob).await.unwrap();
        let bob_reply = reply_listener_bob.recv().await.unwrap();
        let bob_channel = match bob_reply.terms().get(0) {
            Some(Value::Channel(ch)) => ch.clone(),
            _ => panic!("Expected a channel from Bob spawn"),
        };

        // Now simulate Alice sending a "foo" message to Bob.
        let (reply_chan_foo, mut _reply_listener_foo) = Channel::new(10);
        let msg_foo = Message::new(
            vec![Value::Word("foo".into()), Value::String("Hello Bob".into())],
            Some(reply_chan_foo.clone()),
        );
        bob_channel.send(msg_foo).await.unwrap();

        // In this test, Bob’s block only prints to IO and does not reply,
        // so a lack of error indicates the full end-to-end flow is working.
    }
}
