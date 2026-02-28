'use client';

import { BarChart, Bar, XAxis, YAxis, Tooltip, ResponsiveContainer, CartesianGrid } from 'recharts';
import { useTokensByLanguage } from '@/hooks/use-analytics';

export function LanguageChart() {
  const { data = [], isLoading } = useTokensByLanguage();

  const chartData = data.slice(0, 10).map((d) => ({ ext: d.file_ext, tokens: d.input_tokens + d.output_tokens, calls: d.call_count }));

  return (
    <div className="rounded-xl border border-border bg-card p-5 space-y-4">
      <h3 className="font-semibold">Tokens per Language</h3>
      {isLoading ? (
        <div className="h-48 animate-pulse bg-muted rounded" />
      ) : chartData.length === 0 ? (
        <div className="h-48 flex items-center justify-center"><p className="text-muted-foreground text-sm">No language data yet</p></div>
      ) : (
        <ResponsiveContainer width="100%" height={192}>
          <BarChart data={chartData} margin={{ top: 5, right: 10, left: 0, bottom: 5 }}>
            <CartesianGrid strokeDasharray="3 3" stroke="hsl(var(--border))" />
            <XAxis dataKey="ext" tick={{ fontSize: 11 }} />
            <YAxis tick={{ fontSize: 11 }} />
            <Tooltip contentStyle={{ background: 'hsl(var(--card))', border: '1px solid hsl(var(--border))', borderRadius: '8px', fontSize: '12px' }} />
            <Bar dataKey="tokens" fill="#6366f1" radius={[4, 4, 0, 0]} name="Tokens" />
          </BarChart>
        </ResponsiveContainer>
      )}
    </div>
  );
}
