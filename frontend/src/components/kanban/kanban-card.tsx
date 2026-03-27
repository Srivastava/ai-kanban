'use client';

import { useState, useEffect, useRef } from 'react';
import Link from 'next/link';
import { Trash2, FolderOpen, MoreHorizontal } from 'lucide-react';
import { formatDistanceToNow } from 'date-fns';
import { useSortable } from '@dnd-kit/sortable';
import { CSS } from '@dnd-kit/utilities';
import { Button } from '@/components/ui/button';
import { ConfirmDeleteDialog } from '@/components/tasks/confirm-delete-dialog';
import { useDeleteTask, useMoveTask } from '@/hooks/use-tasks';
import { useAllSessions } from '@/hooks/use-sessions';
import type { Task, Stage } from '@/types/task';
import type { CostByTask } from '@/types/analytics';
import { priorityConfig, stageBorderColors, stageCardBg, stageTextColor } from '@/lib/task-colors';

// ── helpers ───────────────────────────────────────────────────────────────────

function stripMarkdown(text: string): string {
  return text
    .replace(/^#{1,6}\s+/gm, '')
    .replace(/\*\*(.+?)\*\*/g, '$1')
    .replace(/\*(.+?)\*/g, '$1')
    .replace(/_(.+?)_/g, '$1')
    .replace(/`(.+?)`/g, '$1')
    .replace(/\[(.+?)\]\(.+?\)/g, '$1')
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

const ALL_STAGES: { value: Stage; label: string }[] = [
  { value: 'backlog', label: 'Backlog' },
  { value: 'planning', label: 'Planning' },
  { value: 'ready', label: 'Ready' },
  { value: 'in_progress', label: 'In Progress' },
  { value: 'review', label: 'Review' },
  { value: 'done', label: 'Done' },
];

// ── component ─────────────────────────────────────────────────────────────────

interface KanbanCardProps {
  task: Task;
  isOverlay?: boolean;
  costData?: CostByTask;
  index?: number;
}

export function KanbanCard({ task, isOverlay = false, costData, index = 0 }: KanbanCardProps) {
  const [confirmOpen, setConfirmOpen] = useState(false);
  const [menuOpen, setMenuOpen] = useState(false);
  const menuRef = useRef<HTMLDivElement>(null);

  const { mutate: deleteTask, isPending: isDeleting } = useDeleteTask();
  const { mutate: moveTask } = useMoveTask();
  const { data: activeSessions = [] } = useAllSessions(['running', 'pending']);

  const { attributes, listeners, setNodeRef, transform, transition, isDragging } =
    useSortable({ id: task.id });

  const style = { transform: CSS.Transform.toString(transform), transition };

  const isSessionActive =
    !!task.session_id &&
    activeSessions.some(
      (s) => s.task_id === task.id && (s.status === 'running' || s.status === 'pending')
    );

  const updatedRelative = formatDistanceToNow(new Date(task.updated_at), { addSuffix: true });
  const updatedAbsolute = new Date(task.updated_at).toLocaleString();
  const folderName = getProjectFolderName(task.project_path);
  const descriptionPreview = task.description ? stripMarkdown(task.description) : null;
  const priorityInfo = task.priority > 0 ? priorityConfig[task.priority] : null;

  const totalTokens = costData
    ? (costData.input_tokens + costData.output_tokens + costData.cache_creation_tokens + costData.cache_read_tokens)
    : 0;
  const fmtTokens = (n: number) =>
    n >= 1_000_000 ? `${(n / 1_000_000).toFixed(1)}M` : n >= 1_000 ? `${(n / 1_000).toFixed(0)}k` : String(n);

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
    <div
      ref={setNodeRef}
      style={style}
      className={`relative group ${isDragging ? 'opacity-20' : ''}`}
    >
      {/* Drag handle wraps the card */}
      <div {...attributes} {...listeners}>
        <Link href={`/tasks/${task.id}`}>
          <div
            className={`
              rounded-xl border border-border/60 overflow-hidden cursor-pointer
              hover:shadow-xl hover:shadow-black/30 hover:-translate-y-0.5
              active:translate-y-0 active:shadow-md
              transition-all duration-150 ease-out
              motion-safe:animate-fade-in-up
              ${stageCardBg[task.stage]} ${stageBorderColors[task.stage]}
              ${isOverlay ? 'shadow-2xl shadow-black/50 scale-[1.04] rotate-[0.7deg] -translate-y-1' : ''}
            `}
            style={{ animationDelay: isOverlay ? undefined : `${index * 35}ms` }}
          >
            {/* Header */}
            <div className="px-3 pt-3 pb-1.5">
              <div className="flex items-start gap-1.5">
                {isSessionActive && (
                  <span
                    className="mt-1 inline-block h-2 w-2 rounded-full bg-green-500 motion-safe:animate-breathe shrink-0"
                    aria-label="Session active"
                  />
                )}
                <p className={`text-sm font-semibold line-clamp-2 flex-1 pr-4 leading-snug ${stageTextColor[task.stage]}`}>
                  {task.title}
                </p>
              </div>
            </div>

            {/* Content */}
            <div className="px-3 pb-3 pt-0 space-y-1.5">
              {descriptionPreview && (
                <p className="text-xs text-muted-foreground line-clamp-2 leading-relaxed">
                  {descriptionPreview}
                </p>
              )}
              {folderName && (
                <span className="inline-flex items-center gap-1 text-[10px] text-muted-foreground bg-black/10 dark:bg-white/5 rounded px-1.5 py-0.5">
                  <FolderOpen className="h-3 w-3" />
                  {folderName}
                </span>
              )}
              {costData && (costData.cost_usd > 0 || totalTokens > 0) && (
                <div className="flex items-center gap-1.5 flex-wrap">
                  {costData.cost_usd > 0 && (
                    <span className="text-[10px] bg-emerald-500/15 text-emerald-700 dark:text-emerald-400 rounded px-1.5 py-0.5 font-medium">
                      ${costData.cost_usd < 0.01 ? '<0.01' : costData.cost_usd.toFixed(2)}
                    </span>
                  )}
                  {totalTokens > 0 && (
                    <span className="text-[10px] bg-primary/10 text-primary rounded px-1.5 py-0.5 font-medium">
                      {fmtTokens(totalTokens)} tok
                    </span>
                  )}
                </div>
              )}
              <div className="flex items-center justify-between pt-0.5">
                <span className="text-[10px] text-muted-foreground" title={updatedAbsolute}>
                  {updatedRelative}
                </span>
                {priorityInfo && (
                  <span className={`inline-flex items-center rounded px-1.5 py-0.5 text-[10px] font-medium ${priorityInfo.className}`}>
                    {priorityInfo.label}
                  </span>
                )}
              </div>
            </div>
          </div>
        </Link>
      </div>

      {!isOverlay && (
        <>
          <Button
            variant="ghost"
            size="icon"
            aria-label="Delete task"
            className="absolute top-1 right-8 h-9 w-9 opacity-0 group-hover:opacity-100 focus-visible:opacity-100 transition-opacity text-destructive hover:bg-destructive/10 z-10"
            onClick={(e) => {
              e.preventDefault();
              e.stopPropagation();
              setConfirmOpen(true);
            }}
          >
            <Trash2 className="h-3 w-3" />
          </Button>

          <div ref={menuRef} className="absolute top-1.5 right-1 z-10">
            <Button
              variant="ghost"
              size="icon"
              aria-label="More options"
              aria-expanded={menuOpen}
              aria-haspopup="menu"
              className="h-9 w-9 opacity-0 group-hover:opacity-100 focus-visible:opacity-100 transition-opacity hover:bg-muted"
              onClick={(e) => {
                e.preventDefault();
                e.stopPropagation();
                setMenuOpen((o) => !o);
              }}
            >
              <MoreHorizontal className="h-3 w-3" />
            </Button>

            {menuOpen && (
              <div
                role="menu"
                aria-label="Task options"
                className="absolute right-0 top-7 min-w-[160px] rounded-md border border-border bg-popover shadow-md z-50 py-1"
              >
                {ALL_STAGES.filter((s) => s.value !== task.stage).map((s) => (
                  <button
                    key={s.value}
                    role="menuitem"
                    className="w-full text-left px-3 py-1.5 text-xs hover:bg-muted transition-colors"
                    onClick={(e) => {
                      e.preventDefault();
                      e.stopPropagation();
                      handleMove(s.value);
                    }}
                  >
                    Move to {s.label}
                  </button>
                ))}
                <div className="h-px bg-border my-1" />
                <Link
                  href={`/tasks/${task.id}`}
                  role="menuitem"
                  className="block px-3 py-1.5 text-xs hover:bg-muted transition-colors"
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
        </>
      )}
    </div>
  );
}
