'use client';

import { Plus } from 'lucide-react';
import { Button } from '@/components/ui/button';

interface HeaderProps {
  onNewTask: () => void;
}

export function Header({ onNewTask }: HeaderProps) {
  return (
    <header className="flex items-center justify-between border-b border-border bg-background px-4 sm:px-6 py-4">
      <div>
        <h1 className="text-3xl font-black tracking-tighter leading-none">Tasks</h1>
        <p className="text-xs text-stage-done-text mt-0.5 font-medium">Manage and track your AI-assisted work</p>
      </div>
      <Button onClick={onNewTask} size="sm">
        <Plus className="mr-1.5 h-4 w-4" />
        New Task
      </Button>
    </header>
  );
}
