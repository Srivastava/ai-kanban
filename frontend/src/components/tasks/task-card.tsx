'use client';

import { useState, useEffect, useRef } from 'react';
import Link from 'next/link';
import { Trash2, FolderOpen, MoreHorizontal } from 'lucide-react';
import { formatDistanceToNow } from 'date-fns';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { ConfirmDeleteDialog } from './confirm-delete-dialog';
import { useDeleteTask, useMoveTask } from '@/hooks/use-tasks';
import { useAllSessions } from '@/hooks/use-sessions';
import type { Task, Stage } from '@/types/task';

// ── helpers ──────────────────────────────────────────────────────────────────

function stripMarkdown(text: string): string {
  return text
    // headings
    .replace(/^#{1,6}\s+/gm, '')
    // bold
    .replace(/\*\*(.+?)\*\*/g, '$1')
    // italic *text* or _text_
    .replace(/\*(.+?)\*/g, '$1')
    .replace(/_(.+?)_/g, '$1')
    // inline code
    .replace(/`(.+?)`/g, '$1')
    // links [text](url)
    .replace(/\[(.+?)\]\(.+?\)/g, '$1')
    // list markers at line start
    .replace(/^[\s]*[-*]\s+/gm, '')
    .replace(/^[\s]*\d+\.\s+/gm, '')
    .trim();
}

function getProjectFolderName(projectPath: string): string {
  if (!projectPath) return '';
  const parts = projectPath.replace(/\/$/, '').split('/');
  return parts[parts.length - 1] || projectPath;
}

// ── constants ─────────────────────────────────────────────────────────────────

const stageColors: Record<Stage, string> = {
  backlog: 'bg-gray-500',
  planning: 'bg-blue-500',
  ready: 'bg-yellow-500',
  in_progress: 'bg-orange-500',
  review: 'bg-purple-500',
  done: 'bg-green-500',
};

const stageBorderColors: Record<Stage, string> = {
  backlog: 'border-l-4 border-l-slate-400',
  planning: 'border-l-4 border-l-blue-500',
  ready: 'border-l-4 border-l-amber-500',
  in_progress: 'border-l-4 border-l-orange-500',
  review: 'border-l-4 border-l-purple-500',
  done: 'border-l-4 border-l-green-500',
};

const stageLabels: Record<Stage, string> = {
  backlog: 'Backlog',
  planning: 'Planning',
  ready: 'Ready',
  in_progress: 'In Progress',
  review: 'Review',
  done: 'Done',
};

const priorityConfig: Record<number, { label: string; className: string }> = {
  1: { label: 'Low', className: 'bg-blue-100 text-blue-700 dark:bg-blue-900/40 dark:text-blue-300' },
  2: { label: 'Medium', className: 'bg-amber-100 text-amber-700 dark:bg-amber-900/40 dark:text-amber-300' },
  3: { label: 'High', className: 'bg-orange-100 text-orange-700 dark:bg-orange-900/40 dark:text-orange-300' },
  4: { label: 'Critical', className: 'bg-red-100 text-red-700 dark:bg-red-900/40 dark:text-red-300' },
};

const MOVE_STAGES: { value: Stage; label: string }[] = [
  { value: 'in_progress', label: 'Move to In Progress' },
  { value: 'review', label: 'Move to Review' },
  { value: 'done', label: 'Move to Done' },
];

// ── component ─────────────────────────────────────────────────────────────────

interface TaskCardProps {
  task: Task;
}

export function TaskCard({ task }: TaskCardProps) {
  const [confirmOpen, setConfirmOpen] = useState(false);
  const [menuOpen, setMenuOpen] = useState(false);
  const menuRef = useRef<HTMLDivElement>(null);

  const { mutate: deleteTask, isPending: isDeleting } = useDeleteTask();
  const { mutate: moveTask } = useMoveTask();
  const { data: activeSessions = [] } = useAllSessions(['running', 'pending']);

  // session activity: task has a session_id AND it's in the running/pending list
  const isSessionActive =
    !!task.session_id &&
    activeSessions.some((s) => s.task_id === task.id && (s.status === 'running' || s.status === 'pending'));

  const updatedRelative = formatDistanceToNow(new Date(task.updated_at), { addSuffix: true });
  const updatedAbsolute = new Date(task.updated_at).toLocaleString();

  const folderName = getProjectFolderName(task.project_path);
  const descriptionPreview = task.description ? stripMarkdown(task.description) : null;

  const priorityInfo = task.priority > 0 ? priorityConfig[task.priority] : null;
  const borderClass = stageBorderColors[task.stage] ?? '';

  // close menu when clicking outside
  useEffect(() => {
    if (!menuOpen) return;
    function handleClickOutside(e: MouseEvent) {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        setMenuOpen(false);
      }
    }
    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, [menuOpen]);

  const handleMove = (stage: Stage) => {
    setMenuOpen(false);
    moveTask({ id: task.id, stage });
  };

  return (
    <div className="relative group">
      <Link href={`/tasks/${task.id}`}>
        <Card className={`hover:shadow-md transition-shadow cursor-pointer ${borderClass} overflow-hidden`}>
          <CardHeader className="pb-2">
            <div className="flex items-start justify-between gap-2">
              <CardTitle className="text-base font-medium line-clamp-2 pr-6 flex items-center gap-1.5">
                {isSessionActive && (
                  <span
                    className="inline-block h-2 w-2 rounded-full bg-green-500 animate-pulse shrink-0"
                    aria-label="Session active"
                  />
                )}
                {task.title}
              </CardTitle>
              <Badge className={`${stageColors[task.stage]} text-white shrink-0`}>
                {stageLabels[task.stage]}
              </Badge>
            </div>
          </CardHeader>
          <CardContent className="space-y-2">
            {descriptionPreview && (
              <p className="text-sm text-muted-foreground line-clamp-2">
                {descriptionPreview}
              </p>
            )}

            {/* project chip */}
            {folderName && (
              <div className="flex items-center gap-1">
                <span className="inline-flex items-center gap-1 text-[10px] text-muted-foreground bg-muted/50 rounded px-1.5 py-0.5">
                  <FolderOpen className="h-3 w-3" />
                  {folderName}
                </span>
              </div>
            )}

            {/* footer */}
            <div className="flex items-center justify-between text-xs text-muted-foreground pt-1">
              <span title={updatedAbsolute}>Updated {updatedRelative}</span>
              {priorityInfo && (
                <span className={`inline-flex items-center rounded px-1.5 py-0.5 text-[10px] font-medium ${priorityInfo.className}`}>
                  {priorityInfo.label}
                </span>
              )}
            </div>
          </CardContent>
        </Card>
      </Link>

      {/* Delete button — floats top-right on hover */}
      <Button
        variant="ghost"
        size="icon"
        aria-label="Delete task"
        className="absolute top-2 right-8 h-7 w-7 opacity-0 group-hover:opacity-100 transition-opacity text-destructive hover:bg-destructive/10 z-10"
        onClick={(e) => { e.preventDefault(); e.stopPropagation(); setConfirmOpen(true); }}
      >
        <Trash2 className="h-3.5 w-3.5" />
      </Button>

      {/* Overflow menu button */}
      <div ref={menuRef} className="absolute top-2 right-2 z-10">
        <Button
          variant="ghost"
          size="icon"
          aria-label="More options"
          className="h-7 w-7 opacity-0 group-hover:opacity-100 transition-opacity hover:bg-muted"
          onClick={(e) => { e.preventDefault(); e.stopPropagation(); setMenuOpen((o) => !o); }}
        >
          <MoreHorizontal className="h-3.5 w-3.5" />
        </Button>

        {menuOpen && (
          <div className="absolute right-0 top-8 min-w-[160px] rounded-md border border-border bg-popover shadow-md z-50 py-1">
            {MOVE_STAGES.filter((s) => s.value !== task.stage).map((s) => (
              <button
                key={s.value}
                className="w-full text-left px-3 py-1.5 text-sm hover:bg-muted transition-colors"
                onClick={(e) => { e.preventDefault(); e.stopPropagation(); handleMove(s.value); }}
              >
                {s.label}
              </button>
            ))}
            <div className="h-px bg-border my-1" />
            <Link
              href={`/tasks/${task.id}`}
              className="block px-3 py-1.5 text-sm hover:bg-muted transition-colors"
              onClick={(e) => e.stopPropagation()}
            >
              Open task
            </Link>
          </div>
        )}
      </div>

      <ConfirmDeleteDialog
        open={confirmOpen}
        onOpenChange={setConfirmOpen}
        title={task.title}
        onConfirm={() => deleteTask(task.id, { onError: () => setConfirmOpen(false) })}
        isDeleting={isDeleting}
      />
    </div>
  );
}
