pub fn build_prompt(task_title: &str, task_description: Option<&str>, stage: &str) -> String {
    let stage_instructions = match stage {
        "planning" => "You are in PLANNING mode. Analyze the task and create a detailed implementation plan. Do NOT make any code changes.",
        "in_progress" => "You are in IN_PROGRESS mode. Implement the task according to the plan. Make code changes as needed.",
        "review" => "You are in REVIEW mode. Review your implementation for bugs and improvements.",
        _ => "Complete the task as appropriate for the current stage.",
    };

    let description_section = task_description
        .map(|d| format!("\n\nTask Description:\n{}", d))
        .unwrap_or_default();

    format!(
        "# Task: {}\n\n{}{}\n\n## Instructions\n- Work on this task in the project context\n- Use available tools as needed\n- Report progress clearly",
        task_title, stage_instructions, description_section
    )
}
