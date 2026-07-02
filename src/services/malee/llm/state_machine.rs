use crate::models::malee::events::UiEvent;
use serde_json::Value;

/// Represents the in-memory state of the agent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentState {
    /// The agent has been initialized and is ready.
    Initialized,
    /// The agent is thinking/processing.
    Thinking,
    /// The agent is currently streaming a text response.
    StreamingResponse {
        /// The accumulated text response.
        text: String,
    },
    /// The agent is executing a tool.
    ToolExecuting {
        /// The name of the tool being executed.
        tool_name: String,
        /// The arguments passed to the tool.
        arguments: Value,
    },
    /// The tool has finished executing.
    ToolExecuted {
        /// The name of the executed tool.
        tool_name: String,
        /// The result returned by the tool.
        result: String,
    },
    /// The agent has successfully completed its turns.
    Completed {
        /// The final text response of the agent.
        final_text: String,
    },
    /// The agent execution failed.
    Failed(String),
}

/// Represents logical agent events that trigger state transitions.
#[derive(Debug)]
pub enum AgentEvent {
    /// Signals the start of a turn in the loop.
    StartTurn,
    /// Signals the receipt of a text token from the LLM.
    ReceiveToken(String),
    /// Signals a request to call a tool.
    CallTool {
        /// The name of the tool.
        name: String,
        /// The tool arguments.
        args: Value,
    },
    /// Signals the result from a tool execution.
    ReceiveToolResult {
        /// The tool result payload.
        result: String,
    },
    /// Signals the successful completion of the agent's turn.
    FinishTurn(String),
    /// Signals an error during execution.
    Error(String),
}

/// The state machine orchestrator for the malee agent.
#[derive(Debug)]
pub struct AgentStateMachine {
    /// The current state of the agent.
    pub state: AgentState,
}

impl AgentStateMachine {
    /// Creates a new `AgentStateMachine` in the `Initialized` state.
    pub const fn new() -> Self {
        Self {
            state: AgentState::Initialized,
        }
    }

    /// Transitions the agent state based on the provided event, returning any emitted UI events.
    pub fn transition(&mut self, event: AgentEvent) -> Vec<UiEvent> {
        let mut ui_events = Vec::new();

        match event {
            AgentEvent::StartTurn => {
                match &self.state {
                    AgentState::Initialized | AgentState::ToolExecuted { .. } => {
                        self.state = AgentState::Thinking;
                    }
                    _ => {
                        // Allow transition to thinking from other states for resilience
                        self.state = AgentState::Thinking;
                    }
                }
            }
            AgentEvent::ReceiveToken(token) => {
                if let AgentState::StreamingResponse { text } = &mut self.state {
                    text.push_str(&token);
                } else {
                    self.state = AgentState::StreamingResponse {
                        text: token.clone(),
                    };
                }
                ui_events.push(UiEvent::Token { text: token });
            }
            AgentEvent::CallTool { name, args } => {
                self.state = AgentState::ToolExecuting {
                    tool_name: name,
                    arguments: args,
                };
            }
            AgentEvent::ReceiveToolResult { result } => {
                let tool_name = match &self.state {
                    AgentState::ToolExecuting { tool_name, .. } => tool_name.clone(),
                    _ => String::new(),
                };
                self.state = AgentState::ToolExecuted { tool_name, result };
            }
            AgentEvent::FinishTurn(final_text) => {
                self.state = AgentState::Completed {
                    final_text: final_text.clone(),
                };
                ui_events.push(UiEvent::AssistantMessageDone {
                    full_text: final_text,
                });
            }
            AgentEvent::Error(message) => {
                self.state = AgentState::Failed(message.clone());
                let code = if message.contains("turns") || message.contains("depth") {
                    "LOOP_DEPTH".to_string()
                } else {
                    "AGENT_ERROR".to_string()
                };
                ui_events.push(UiEvent::Error {
                    code,
                    message,
                    recoverable: true,
                });
            }
        }

        ui_events
    }
}

impl Default for AgentStateMachine {
    fn default() -> Self {
        Self::new()
    }
}
