/// Build the prompt sent to Claude.
///
/// Ordering is intentional for prompt cache efficiency:
///   1. Task identity + description  (most stable — same across all sessions for this task)
///   2. Stage instructions           (changes at most a few times per task)
///   3. Conversation context         (dynamic — changes every session, so placed last)
///
/// Putting stable content first maximises prefix-cache hits when sessions share the same task.
///
/// `has_plan` must be set by the caller based on `task.instructions.is_some()`.
/// When true, the in_progress prompt tells Claude to read `.claude/ai-kanban-plan.md`
/// instead of re-embedding the description in the prompt.
pub fn build_prompt(
    task_title: &str,
    task_description: Option<&str>,
    stage: &str,
    conversation_context: Option<&str>,
    has_plan: bool,
) -> String {
    let stage_instructions = match stage {
        "planning" => concat!(
            "You are in PLANNING mode.\n",
            "1. Analyse the task thoroughly — read the codebase, understand the context.\n",
            "2. Write a detailed, actionable implementation plan.\n",
            "3. Save the plan to `.claude/ai-kanban-plan.md` in the project root ",
            "(create the `.claude/` directory if needed). Use markdown headings and a checklist.\n",
            "4. Do NOT make any code changes — planning only.",
        ),
        "in_progress" if has_plan => concat!(
            "You are in IN_PROGRESS mode.\n",
            "The implementation plan is in `.claude/ai-kanban-plan.md` — read it first, then implement it.\n",
            "Work through the checklist items in order. Make all necessary code changes.",
        ),
        "in_progress" => concat!(
            "You are in IN_PROGRESS mode.\n",
            "Implement the task according to the description. Make all necessary code changes.",
        ),
        "review" if has_plan => concat!(
            "You are in REVIEW mode.\n",
            "Review your implementation for correctness, bugs, and edge cases. ",
            "Check that all items in `.claude/ai-kanban-plan.md` are complete. ",
            "Fix any issues you find. Do not add unrelated changes.",
        ),
        "review" => concat!(
            "You are in REVIEW mode.\n",
            "Review your implementation for correctness, bugs, and edge cases. ",
            "Fix any issues you find. Do not add unrelated changes.",
        ),
        _ => "Complete the task as appropriate for the current stage.",
    };

    // Static section — always present, same across sessions for this task
    let description_section = task_description
        .map(|d| format!("\n\n## Task Description\n{d}"))
        .unwrap_or_default();

    // Dynamic section — only appended when there is new context (kept last for cache)
    let conversation_section = conversation_context
        .map(|c| format!("\n\n## Context\n{c}"))
        .unwrap_or_default();

    format!(
        "# Task: {task_title}{description_section}\n\n## Stage\n{stage_instructions}\n\n## Guidelines\n- Work on this task in the project context\n- Use available tools as needed\n- Report progress and findings clearly{conversation_section}",
    )
}
