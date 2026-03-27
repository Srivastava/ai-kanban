'use client';

import { useState, useEffect, useRef } from 'react';
import Link from 'next/link';
import { Trash2, FolderOpen, MoreHorizontal } from 'lucide-react';
import { formatDistanceToNow } from 'date-fns';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { ConfirmDeleteDialog } from './confirm-delete-dialog';
import { useDeleteTask, useMoveTask } from '@/hooks/use-tasks';
import { useAllSessions } from '@/hooks/use-sessions';
import type { Task, Stage } from '@/types/task';
import { stageBorderColors, stageLabels, priorityConfig, stageCardBg, stageTextColor, stageChipConfig } from '@/lib/task-colors';

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
  const cardBg = stageCardBg[task.stage] ?? '';

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
        <div className={`
          rounded-xl border border-border/60 overflow-hidden cursor-pointer
          hover:shadow-xl hover:shadow-black/25 hover:-translate-y-0.5
          active:translate-y-0 active:shadow-md
          transition-all duration-150 ease-out
          ${cardBg} ${borderClass}
        `}>
          {/* Header */}
          <div className="p-4 pb-2">
            <div className="flex items-start justify-between gap-2">
              <p className={`text-base font-semibold line-clamp-2 pr-6 flex items-center gap-1.5 leading-snug ${stageTextColor[task.stage]}`}>
                {isSessionActive && (
                  <span
                    className="inline-block h-2 w-2 rounded-full bg-green-500 motion-safe:animate-breathe shrink-0"
                    aria-label="Session active"
                  />
                )}
                {task.title}
              </p>
              <Badge className={`${stageChipConfig[task.stage].className} shrink-0 text-[10px] border font-semibold`}>
                {stageLabels[task.stage]}
              </Badge>
            </div>
          </div>

          {/* Content */}
          <div className="px-4 pb-4 space-y-2">
            {descriptionPreview && (
              <p className="text-sm text-muted-foreground line-clamp-2 leading-relaxed">
                {descriptionPreview}
              </p>
            )}

            {/* project chip */}
            {folderName && (
              <span className="inline-flex items-center gap-1 text-[10px] text-muted-foreground bg-black/10 dark:bg-white/5 rounded px-1.5 py-0.5">
                <FolderOpen className="h-3 w-3" />
                {folderName}
              </span>
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
          </div>
        </div>
      </Link>

      {/* Delete button — floats top-right on hover */}
      <Button
        variant="ghost"
        size="icon"
        aria-label="Delete task"
        className="absolute top-2 right-9 h-9 w-9 opacity-0 group-hover:opacity-100 focus-visible:opacity-100 transition-opacity text-destructive hover:bg-destructive/10 z-10"
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
          aria-expanded={menuOpen}
          aria-haspopup="menu"
          className="h-9 w-9 opacity-0 group-hover:opacity-100 focus-visible:opacity-100 transition-opacity hover:bg-muted"
          onClick={(e) => { e.preventDefault(); e.stopPropagation(); setMenuOpen((o) => !o); }}
        >
          <MoreHorizontal className="h-3.5 w-3.5" />
        </Button>

        {menuOpen && (
          <div
            role="menu"
            aria-label="Task options"
            className="absolute right-0 top-8 min-w-[160px] rounded-md border border-border bg-popover shadow-md z-50 py-1"
          >
            {MOVE_STAGES.filter((s) => s.value !== task.stage).map((s) => (
              <button
                key={s.value}
                role="menuitem"
                className="w-full text-left px-3 py-1.5 text-sm hover:bg-muted transition-colors"
                onClick={(e) => { e.preventDefault(); e.stopPropagation(); handleMove(s.value); }}
              >
                {s.label}
              </button>
            ))}
            <div className="h-px bg-border my-1" />
            <Link
              href={`/tasks/${task.id}`}
              role="menuitem"
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
