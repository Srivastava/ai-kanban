pub mod jsonl_parser;
mod manager;
mod prompts;
mod queue;

pub use manager::{
    generate_claude_handover, write_task_context_file, ClaudeEvent, ClaudeManager,
    COMPRESSION_TOKEN_THRESHOLD,
};
pub use prompts::build_prompt;
pub use queue::{QueuedTask, SessionQueue};
