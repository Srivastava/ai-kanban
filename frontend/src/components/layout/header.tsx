'use client';

import { Plus } from 'lucide-react';
import { Button } from '@/components/ui/button';

interface HeaderProps {
  onNewTask: () => void;
}

export function Header({ onNewTask }: HeaderProps) {
  return (
    <header className="flex h-14 items-center justify-between border-b border-border bg-background px-4 sm:px-6">
      <h1 className="text-xl font-semibold">AI Kanban</h1>
      <Button onClick={onNewTask}>
        <Plus className="mr-2 h-4 w-4" />
        New Task
      </Button>
    </header>
  );
}
