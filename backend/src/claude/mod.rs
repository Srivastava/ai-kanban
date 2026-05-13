pub mod jsonl_parser;
mod manager;
mod prompts;
mod queue;

pub use manager::{
    generate_claude_handover, write_task_context_file, ClaudeEvent, ClaudeManager,
    CONTEXT_HANDOVER_THRESHOLD, CONTEXT_WARN_THRESHOLD,
};
pub use prompts::build_prompt;
pub use queue::{ManagesSession, QueuedTask, SessionQueue};
