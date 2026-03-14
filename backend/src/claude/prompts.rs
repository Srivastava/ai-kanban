/// Build the prompt sent to Claude.
///
/// Ordering is intentional for prompt cache efficiency:
///   1. Task identity + description  (most stable — same across all sessions for this task)
///   2. Stage instructions           (changes at most a few times per task)
///   3. Conversation context         (dynamic — changes every session, so placed last)
///
/// Putting stable content first maximises prefix-cache hits when sessions share the same task.
pub fn build_prompt(
    task_title: &str,
    task_description: Option<&str>,
    stage: &str,
    conversation_context: Option<&str>,
) -> String {
    let stage_instructions = match stage {
        "planning" => "You are in PLANNING mode. Analyse the task and create a detailed implementation plan. Do NOT make any code changes yet.",
        "in_progress" => "You are in IN_PROGRESS mode. Implement the task according to the plan. Make code changes as needed.",
        "review" => "You are in REVIEW mode. Review your implementation for bugs and improvements. Fix any issues you find.",
        _ => "Complete the task as appropriate for the current stage.",
    };

    // Static section — always present, never changes between sessions
    let description_section = task_description
        .map(|d| format!("\n\n## Task Description\n{}", d))
        .unwrap_or_default();

    // Dynamic section — only appended when there is new context (kept last for cache)
    let conversation_section = conversation_context
        .map(|c| format!("\n\n## Context\n{}", c))
        .unwrap_or_default();

    format!(
        "# Task: {task_title}{description_section}\n\n## Stage\n{stage_instructions}\n\n## Guidelines\n- Work on this task in the project context\n- Use available tools as needed\n- Report progress clearly{conversation_section}",
    )
}
