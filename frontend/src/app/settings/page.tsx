import { Sidebar } from '@/components/layout/sidebar';
import { FeatureFlagsPanel } from '@/components/settings/feature-flags-panel';

export default function SettingsPage() {
  return (
    <div className="flex min-h-screen bg-background">
      <Sidebar />
      <div className="flex-1 p-4 sm:p-8 pb-20 md:pb-8 max-w-2xl min-w-0">
        <h1 className="text-3xl font-black tracking-tighter leading-none mb-1">Settings</h1>
        <p className="text-xs text-muted-foreground mb-10">
          Configure LiteLLM-powered optimizations to reduce Claude token usage.
        </p>

        <section>
          <h2 className="text-sm font-bold uppercase tracking-widest text-primary/70 mb-1">LiteLLM Features</h2>
          <p className="text-xs text-muted-foreground mb-4">
            These features use your configured LiteLLM endpoint (
            <span className="font-mono">LITELLM_BASE_URL</span>) to offload
            non-critical tasks from Claude, reducing token consumption.
          </p>
          <FeatureFlagsPanel />
        </section>
      </div>
    </div>
  );
}
