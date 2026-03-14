mod manager;
pub mod jsonl_parser;
mod prompts;
mod queue;

pub use manager::{ClaudeManager, ClaudeEvent, write_task_context_file};
pub use prompts::build_prompt;
pub use queue::{QueuedTask, SessionQueue};
