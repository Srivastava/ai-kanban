'use client';

import { useFeatureFlags, useUpdateFeatureFlag } from '@/hooks/use-settings';

const FLAG_META: Record<string, { label: string; description: string }> = {
  litellm_session_summary: {
    label: 'Session Summary',
    description: 'After each Claude session, LiteLLM generates a concise summary posted to the Updates section.',
  },
  litellm_context_compression: {
    label: 'Context Compression',
    description: `When a session's input tokens exceed 150K, LiteLLM compresses the session context and stores it on the task. Future sessions start with the compressed context instead of re-reading the full history.`,
  },
  litellm_pre_session_briefing: {
    label: 'Pre-session Briefing',
    description: 'Before each "Continue Session", LiteLLM condenses the conversation history into a compact briefing, reducing the tokens Claude needs to process.',
  },
  litellm_task_enrichment: {
    label: 'Task Description Enrichment',
    description: 'Before the very first Claude session on a task, LiteLLM expands a terse title/description into a structured, actionable brief with acceptance criteria and focus areas.',
  },
};

export function FeatureFlagsPanel() {
  const { data: flags = [], isLoading } = useFeatureFlags();
  const { mutate: updateFlag, isPending } = useUpdateFeatureFlag();

  if (isLoading) {
    return (
      <div className="space-y-4">
        {[...Array(4)].map((_, i) => (
          <div key={i} className="h-16 animate-pulse bg-muted rounded-lg" />
        ))}
      </div>
    );
  }

  return (
    <div className="space-y-3">
      {flags.map((flag) => {
        const meta = FLAG_META[flag.key];
        return (
          <div
            key={flag.key}
            className="flex items-start gap-4 rounded-lg border border-border p-4"
          >
            <button
              role="switch"
              aria-checked={flag.enabled}
              disabled={isPending}
              onClick={() => updateFlag({ key: flag.key, enabled: !flag.enabled })}
              className={`relative mt-0.5 h-5 w-9 shrink-0 rounded-full transition-colors focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2 disabled:opacity-50 ${
                flag.enabled ? 'bg-primary' : 'bg-muted-foreground/30'
              }`}
            >
              <span
                className={`absolute top-0.5 left-0.5 h-4 w-4 rounded-full bg-white shadow transition-transform ${
                  flag.enabled ? 'translate-x-4' : 'translate-x-0'
                }`}
              />
            </button>
            <div className="min-w-0 flex-1">
              <p className="text-sm font-medium">
                {meta?.label ?? flag.key}
              </p>
              {meta?.description && (
                <p className="text-xs text-muted-foreground mt-0.5">
                  {meta.description}
                </p>
              )}
              <p className="text-xs text-muted-foreground/60 mt-1 font-mono">
                {flag.key}
              </p>
            </div>
          </div>
        );
      })}
    </div>
  );
}
