// Extracted Malee unit tests
use nadzu::models::malee::cart::{CartItem, CartState};
use nadzu::models::malee::checkout::CheckoutDraft;
use nadzu::models::malee::events::UiEvent;
use nadzu::models::malee::session::SessionState;
use nadzu::services::malee::connector::client::MaleeConnector;
use nadzu::services::malee::connector::tools::*;

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod services_malee_cart_reducer_tests {
    use super::*;
    #[allow(unused_imports)]
    use nadzu::services::malee::cart::reducer::*;

    #[test]
    fn test_add_item() {
        let state = CartState::default();
        let action = CartAction::AddItem {
            product: CartItem {
                product_id: "p1".to_string(),
                name: "Test".to_string(),
                price_lkr: 1000,
                quantity: 1,
                image_url: None,
                is_perishable: false,
            },
        };
        let new_state = reduce(state, action, 10).unwrap();
        assert_eq!(new_state.items.len(), 1);
        assert_eq!(new_state.items[0].quantity, 1);
    }
}

#[cfg(test)]
mod services_malee_checkout_validate_tests {
    use super::*;
    #[allow(unused_imports)]
    use nadzu::services::malee::checkout::validate::*;

    #[test]
    fn test_validate_empty() {
        let draft = CheckoutDraft::default();
        let cart = CartState::default();
        let res = validate(&draft, &cart);
        assert!(res.is_err());
    }
}

#[cfg(test)]
mod services_malee_language_normalize_tests {
    #[allow(unused_imports)]
    use nadzu::services::malee::language::normalize::*;

    #[test]
    fn test_normalize_budget() {
        let hints = normalize("gift for amma under 5000 in colombo");
        assert_eq!(hints.inferred_recipient.as_deref(), Some("mother"));
        assert_eq!(hints.inferred_budget_max_lkr, Some(5000));
        assert_eq!(hints.inferred_city_hint.as_deref(), Some("colombo"));
    }
}

#[cfg(test)]
#[allow(clippy::panic)]
mod services_malee_llm_state_machine_tests {
    use super::*;
    #[allow(unused_imports)]
    use nadzu::services::malee::llm::state_machine::*;

    use serde_json::json;

    #[test]
    fn test_initial_state() {
        let sm = AgentStateMachine::new();
        assert_eq!(sm.state, AgentState::Initialized);
    }

    #[test]
    fn test_start_turn_transition() {
        let mut sm = AgentStateMachine::new();
        let events = sm.transition(AgentEvent::StartTurn);
        assert!(events.is_empty());
        assert_eq!(sm.state, AgentState::Thinking);
    }

    #[test]
    fn test_streaming_tokens() {
        let mut sm = AgentStateMachine::new();
        sm.transition(AgentEvent::StartTurn);

        let events1 = sm.transition(AgentEvent::ReceiveToken("Hello".to_string()));
        assert_eq!(events1.len(), 1);
        if let UiEvent::Token { text } = &events1[0] {
            assert_eq!(text, "Hello");
        } else {
            panic!("Expected UiEvent::Token");
        }
        assert_eq!(
            sm.state,
            AgentState::StreamingResponse {
                text: "Hello".to_string()
            }
        );

        let events2 = sm.transition(AgentEvent::ReceiveToken(" World".to_string()));
        assert_eq!(events2.len(), 1);
        if let UiEvent::Token { text } = &events2[0] {
            assert_eq!(text, " World");
        } else {
            panic!("Expected UiEvent::Token");
        }
        assert_eq!(
            sm.state,
            AgentState::StreamingResponse {
                text: "Hello World".to_string()
            }
        );
    }

    #[test]
    fn test_tool_calling_flow() {
        let mut sm = AgentStateMachine::new();
        sm.transition(AgentEvent::StartTurn);

        let args = json!({"item": "apple"});
        let events1 = sm.transition(AgentEvent::CallTool {
            name: "add_to_cart".to_string(),
            args: args.clone(),
        });
        assert!(events1.is_empty());
        assert_eq!(
            sm.state,
            AgentState::ToolExecuting {
                tool_name: "add_to_cart".to_string(),
                arguments: args,
            }
        );

        let events2 = sm.transition(AgentEvent::ReceiveToolResult {
            result: "Success".to_string(),
        });
        assert!(events2.is_empty());
        assert_eq!(
            sm.state,
            AgentState::ToolExecuted {
                tool_name: "add_to_cart".to_string(),
                result: "Success".to_string(),
            }
        );

        sm.transition(AgentEvent::StartTurn);
        assert_eq!(sm.state, AgentState::Thinking);
    }

    #[test]
    fn test_finish_turn() {
        let mut sm = AgentStateMachine::new();
        sm.transition(AgentEvent::StartTurn);
        sm.transition(AgentEvent::ReceiveToken("Done".to_string()));
        let events = sm.transition(AgentEvent::FinishTurn("Done".to_string()));
        assert_eq!(events.len(), 1);
        if let UiEvent::AssistantMessageDone { full_text } = &events[0] {
            assert_eq!(full_text, "Done");
        } else {
            panic!("Expected UiEvent::AssistantMessageDone");
        }
        assert_eq!(
            sm.state,
            AgentState::Completed {
                final_text: "Done".to_string()
            }
        );
    }

    #[test]
    fn test_error_transition() {
        let mut sm = AgentStateMachine::new();
        let events = sm.transition(AgentEvent::Error("Agent took too many turns".to_string()));
        assert_eq!(events.len(), 1);
        if let UiEvent::Error {
            code,
            message,
            recoverable,
        } = &events[0]
        {
            assert_eq!(code, "LOOP_DEPTH");
            assert_eq!(message, "Agent took too many turns");
            assert!(recoverable);
        } else {
            panic!("Expected UiEvent::Error");
        }
        assert_eq!(
            sm.state,
            AgentState::Failed("Agent took too many turns".to_string())
        );
    }

    #[test]
    fn test_streaming_tokens_in_place_optimization() {
        let mut sm = AgentStateMachine::new();
        sm.transition(AgentEvent::StartTurn);

        // Transition from initial state (Thinking) to StreamingResponse
        sm.transition(AgentEvent::ReceiveToken("First".to_string()));
        if let AgentState::StreamingResponse { text } = &sm.state {
            assert_eq!(text, "First");
        } else {
            panic!("Expected StreamingResponse");
        }

        // Transition while already in StreamingResponse state
        sm.transition(AgentEvent::ReceiveToken("Second".to_string()));
        if let AgentState::StreamingResponse { text } = &sm.state {
            assert_eq!(text, "FirstSecond");
        } else {
            panic!("Expected StreamingResponse");
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod services_malee_llm_tools_tests {
    use super::*;
    #[allow(unused_imports)]
    use nadzu::services::malee::llm::tools::*;

    use nadzu::models::malee::cart::CartState;
    use nadzu::models::malee::checkout::CheckoutDraft;
    use nadzu::models::malee::profile::{SessionContext, UserProfile};
    use nadzu::models::malee::session::LanguageMode;
    use uuid::Uuid;

    fn create_test_session() -> SessionState {
        SessionState {
            session_id: Uuid::new_v4(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            language_mode: LanguageMode::Auto,
            conversation_history: vec![],
            user_profile: UserProfile::default(),
            session_context: SessionContext::default(),
            cart: CartState::default(),
            checkout_draft: CheckoutDraft::default(),
            last_products: vec![],
            order_last_created_at: None,
            active_llm_index: 0,
        }
    }

    fn create_test_connector() -> MaleeConnector {
        MaleeConnector::new(reqwest::Client::new(), "http://localhost".to_string(), 1000)
    }

    #[tokio::test]
    async fn test_execute_add_to_cart() {
        let mut session = create_test_session();
        let connector = create_test_connector();
        let args = serde_json::json!({
            "product_id": "prod-1",
            "name": "Cake",
            "price_lkr": 1500,
            "quantity": 2,
            "image_url": null,
            "is_perishable": false
        });

        let (res, out) = execute_tool(
            &mut session,
            &connector,
            TOOL_ADD_TO_CART,
            args,
            "session-1",
        )
        .await
        .unwrap();

        match res {
            ToolResult::AddToCart { item_name } => {
                assert_eq!(item_name, "Cake");
            }
            _ => panic!("Expected AddToCart"),
        }
        assert_eq!(out, "Added Cake to cart");
        assert_eq!(session.cart.items.len(), 1);
        assert_eq!(session.cart.items[0].product_id, "prod-1");
    }

    #[tokio::test]
    async fn test_execute_remove_from_cart() {
        let mut session = create_test_session();
        let connector = create_test_connector();

        // Populate cart first
        session.cart.items.push(CartItem {
            product_id: "prod-1".to_string(),
            name: "Cake".to_string(),
            price_lkr: 1500,
            quantity: 2,
            image_url: None,
            is_perishable: false,
        });

        let args = serde_json::json!({
            "product_id": "prod-1"
        });

        let (res, out) = execute_tool(
            &mut session,
            &connector,
            TOOL_REMOVE_FROM_CART,
            args,
            "session-1",
        )
        .await
        .unwrap();

        match res {
            ToolResult::RemoveFromCart { product_id } => {
                assert_eq!(product_id, "prod-1");
            }
            _ => panic!("Expected RemoveFromCart"),
        }
        assert_eq!(out, "Removed prod-1 from cart");
        assert!(session.cart.items.is_empty());
    }

    #[tokio::test]
    async fn test_execute_set_quantity() {
        let mut session = create_test_session();
        let connector = create_test_connector();

        session.cart.items.push(CartItem {
            product_id: "prod-1".to_string(),
            name: "Cake".to_string(),
            price_lkr: 1500,
            quantity: 2,
            image_url: None,
            is_perishable: false,
        });

        let args = serde_json::json!({
            "product_id": "prod-1",
            "quantity": 5
        });

        let (res, out) = execute_tool(
            &mut session,
            &connector,
            TOOL_SET_QUANTITY,
            args,
            "session-1",
        )
        .await
        .unwrap();

        match res {
            ToolResult::SetQuantity {
                product_id,
                quantity,
            } => {
                assert_eq!(product_id, "prod-1");
                assert_eq!(quantity, 5);
            }
            _ => panic!("Expected SetQuantity"),
        }
        assert_eq!(out, "Set quantity of prod-1 to 5");
        assert_eq!(session.cart.items[0].quantity, 5);
    }

    #[tokio::test]
    async fn test_execute_clear_cart() {
        let mut session = create_test_session();
        let connector = create_test_connector();

        session.cart.items.push(CartItem {
            product_id: "prod-1".to_string(),
            name: "Cake".to_string(),
            price_lkr: 1500,
            quantity: 2,
            image_url: None,
            is_perishable: false,
        });

        let (res, out) = execute_tool(
            &mut session,
            &connector,
            TOOL_CLEAR_CART,
            serde_json::Value::Null,
            "session-1",
        )
        .await
        .unwrap();

        match res {
            ToolResult::ClearCart => {}
            _ => panic!("Expected ClearCart"),
        }
        assert_eq!(out, "Cleared cart");
        assert!(session.cart.items.is_empty());
    }

    #[tokio::test]
    async fn test_execute_setup_delivery() {
        let mut session = create_test_session();
        let connector = create_test_connector();
        connector.internal_inject_city_cache("Colombo", vec!["Colombo".to_string()]);

        let args = serde_json::json!({
            "city": "Colombo",
            "date": "2026-12-25"
        });

        let (res, out) = execute_tool(
            &mut session,
            &connector,
            TOOL_SETUP_DELIVERY,
            args,
            "session-1",
        )
        .await
        .unwrap();

        match res {
            ToolResult::SetupDelivery => {}
            _ => panic!("Expected SetupDelivery"),
        }
        assert_eq!(out, "Updated delivery info");
        assert_eq!(
            session.checkout_draft.delivery.as_ref().unwrap().city,
            "Colombo"
        );
        assert_eq!(
            session
                .checkout_draft
                .delivery
                .as_ref()
                .unwrap()
                .date
                .to_string(),
            "2026-12-25"
        );
    }

    #[tokio::test]
    async fn test_execute_save_user_fact() {
        let mut session = create_test_session();
        let connector = create_test_connector();

        let args = serde_json::json!({
            "fact": "Loves chocolate cake"
        });

        let (res, out) = execute_tool(
            &mut session,
            &connector,
            TOOL_SAVE_USER_FACT,
            args,
            "session-1",
        )
        .await
        .unwrap();

        match res {
            ToolResult::SaveUserFact => {}
            _ => panic!("Expected SaveUserFact"),
        }
        assert_eq!(out, "Fact saved successfully");
        assert_eq!(
            session.user_profile.memories,
            vec!["Loves chocolate cake".to_string()]
        );
    }

    #[tokio::test]
    async fn test_execute_update_user_profile() {
        let mut session = create_test_session();
        let connector = create_test_connector();

        let args = serde_json::json!({
            "first_name": "Nadun",
            "email": "nadun@example.com",
            "favorite_categories": ["Cakes", "Flowers"]
        });

        let (res, out) = execute_tool(
            &mut session,
            &connector,
            TOOL_UPDATE_USER_PROFILE,
            args,
            "session-1",
        )
        .await
        .unwrap();

        match res {
            ToolResult::UpdateUserProfile => {}
            _ => panic!("Expected UpdateUserProfile"),
        }
        assert_eq!(out, "Profile updated successfully");
        assert_eq!(session.user_profile.first_name.as_deref(), Some("Nadun"));
        assert_eq!(
            session.user_profile.email.as_deref(),
            Some("nadun@example.com")
        );
        assert_eq!(
            session.user_profile.favorite_categories,
            vec!["Cakes".to_string(), "Flowers".to_string()]
        );
    }
}

#[cfg(test)]
mod services_malee_sse_parser_tests {
    #[allow(unused_imports)]
    use nadzu::services::malee::sse::parser::*;

    #[test]
    fn test_parse_sse_line_standard() {
        assert_eq!(parse_sse_line("data: hello"), Some("hello"));
        assert_eq!(parse_sse_line("data:  trimmed "), Some("trimmed"));
    }

    #[test]
    fn test_parse_sse_line_done() {
        assert_eq!(parse_sse_line("data: [DONE]"), Some("[DONE]"));
    }

    #[test]
    fn test_parse_sse_line_invalid() {
        assert_eq!(parse_sse_line("event: message"), None);
        assert_eq!(parse_sse_line("data:[DONE]"), None); // missing space after colon
    }
}
