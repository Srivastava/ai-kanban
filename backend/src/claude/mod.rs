mod manager;
pub mod jsonl_parser;
mod prompts;
mod queue;

pub use manager::{ClaudeManager, SessionOutput};
pub use prompts::build_prompt;
pub use queue::{QueuedTask, SessionQueue};
