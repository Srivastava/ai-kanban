'use client';

import { useState, useMemo } from 'react';
import Link from 'next/link';
import { LayoutGrid, List, AlignJustify, ChevronDown, FolderOpen, Plus } from 'lucide-react';
import { formatDistanceToNow } from 'date-fns';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { TaskCard } from './task-card';
import { TaskCardSkeleton } from './task-card-skeleton';
import type { Task, Stage } from '@/types/task';
import { stageLabels, stageChipConfig } from '@/lib/task-colors';

// ── helpers ───────────────────────────────────────────────────────────────────

function getProjectFolderName(projectPath: string): string {
  if (!projectPath) return '';
  const parts = projectPath.replace(/\/$/, '').split('/');
  return parts[parts.length - 1] || projectPath;
}

type ViewMode = 'grid' | 'list' | 'compact';
type SortMode = 'newest' | 'oldest' | 'title';

// ── sub-components ────────────────────────────────────────────────────────────

function ListRow({ task }: { task: Task }) {
  const updatedRelative = formatDistanceToNow(new Date(task.updated_at), { addSuffix: true });
  const updatedAbsolute = new Date(task.updated_at).toLocaleString();
  const folderName = getProjectFolderName(task.project_path);

  return (
    <Link href={`/tasks/${task.id}`}>
      <div className="flex items-center gap-3 rounded-md border border-border bg-card hover:bg-muted/40 transition-colors px-4 py-3 cursor-pointer min-h-[48px]">
        <div className="flex-1 min-w-0">
          <span className="text-sm font-medium truncate block">{task.title}</span>
          {folderName && (
            <span className="inline-flex items-center gap-1 text-[10px] text-muted-foreground mt-0.5">
              <FolderOpen className="h-3 w-3" />
              {folderName}
            </span>
          )}
        </div>
        <Badge className={`${stageChipConfig[task.stage].className} shrink-0 text-xs border font-semibold`}>
          {stageLabels[task.stage]}
        </Badge>
        <span
          className="text-xs text-muted-foreground shrink-0 hidden sm:block"
          title={updatedAbsolute}
        >
          {updatedRelative}
        </span>
      </div>
    </Link>
  );
}

function CompactRow({ task }: { task: Task }) {
  const updatedRelative = formatDistanceToNow(new Date(task.updated_at), { addSuffix: true });
  const updatedAbsolute = new Date(task.updated_at).toLocaleString();

  return (
    <Link href={`/tasks/${task.id}`}>
      <div className="flex items-center gap-3 border-b border-border hover:bg-muted/30 transition-colors px-3 py-2 cursor-pointer text-sm min-h-[48px]">
        <span className="flex-1 min-w-0 truncate font-medium">{task.title}</span>
        <Badge className={`${stageChipConfig[task.stage].className} text-[10px] py-0 px-1.5 shrink-0 border font-semibold`}>
          {stageLabels[task.stage]}
        </Badge>
        <span
          className="text-xs text-muted-foreground shrink-0 hidden sm:block"
          title={updatedAbsolute}
        >
          {updatedRelative}
        </span>
      </div>
    </Link>
  );
}

// ── empty state ───────────────────────────────────────────────────────────────

function EmptyState({ stageParam, onNewTask }: { stageParam?: string; onNewTask?: () => void }) {
  const stageLabel = stageParam
    ? stageLabels[stageParam as Stage] ?? stageParam
    : null;

  const messages: Record<string, { title: string; hint: string }> = {
    backlog:     { title: 'Nothing queued up',   hint: 'Capture ideas and future work here.' },
    planning:    { title: 'No tasks in planning', hint: 'Move tasks here when the AI starts breaking them down.' },
    ready:       { title: 'Nothing ready yet',    hint: 'Tasks move here when they\'re fully scoped and ready to execute.' },
    in_progress: { title: 'Nothing active',       hint: 'Start an AI session on a task to move it here.' },
    review:      { title: 'No tasks in review',   hint: 'Tasks land here when the AI finishes and needs a human eye.' },
    done:        { title: 'Nothing done yet',      hint: 'Completed tasks will show up here.' },
  };

  const msg = stageParam && messages[stageParam]
    ? messages[stageParam]
    : { title: 'No tasks yet', hint: 'Create your first task to get started.' };

  return (
    <div className="flex flex-col items-center justify-center h-64 gap-4 text-center px-6">
      <div className="h-14 w-14 rounded-2xl border-2 border-dashed border-border flex items-center justify-center">
        <LayoutGrid className="h-6 w-6 text-muted-foreground/40" aria-hidden="true" />
      </div>
      <div className="space-y-1">
        <p className="text-sm font-semibold text-foreground">{msg.title}</p>
        <p className="text-sm text-muted-foreground">{msg.hint}</p>
        {stageLabel && (
          <p className="text-xs text-muted-foreground/60 mt-1">
            Showing <span className="font-medium">{stageLabel}</span>
          </p>
        )}
      </div>
      {onNewTask && (
        <Button size="sm" onClick={onNewTask}>
          <Plus className="h-4 w-4 mr-1" aria-hidden="true" />
          New Task
        </Button>
      )}
    </div>
  );
}

// ── main component ────────────────────────────────────────────────────────────

interface TaskListProps {
  tasks: Task[];
  isLoading: boolean;
  stageParam?: string;
  onNewTask?: () => void;
}

export function TaskList({ tasks, isLoading, stageParam, onNewTask }: TaskListProps) {
  const [viewMode, setViewMode] = useState<ViewMode>('grid');
  const [sortMode, setSortMode] = useState<SortMode>('newest');
  const [projectFilter, setProjectFilter] = useState<string>('all');
  const [sortOpen, setSortOpen] = useState(false);
  const [projectOpen, setProjectOpen] = useState(false);

  // unique project paths
  const projectPaths = useMemo(() => {
    const paths = Array.from(new Set(tasks.map((t) => t.project_path).filter(Boolean)));
    return paths.sort();
  }, [tasks]);

  // filter + sort
  const filteredSorted = useMemo(() => {
    let list = tasks;

    if (projectFilter !== 'all') {
      list = list.filter((t) => t.project_path === projectFilter);
    }

    if (sortMode === 'newest') {
      list = [...list].sort((a, b) => new Date(b.updated_at).getTime() - new Date(a.updated_at).getTime());
    } else if (sortMode === 'oldest') {
      list = [...list].sort((a, b) => new Date(a.updated_at).getTime() - new Date(b.updated_at).getTime());
    } else if (sortMode === 'title') {
      list = [...list].sort((a, b) => a.title.localeCompare(b.title));
    }

    return list;
  }, [tasks, projectFilter, sortMode]);

  const sortLabels: Record<SortMode, string> = {
    newest: 'Newest',
    oldest: 'Oldest',
    title: 'Title A-Z',
  };

  if (isLoading) {
    return (
      <div className="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-3">
        {Array.from({ length: 6 }).map((_, i) => (
          <TaskCardSkeleton key={i} />
        ))}
      </div>
    );
  }

  return (
    <div className="space-y-4">
      {/* Controls row */}
      <div className="flex flex-wrap items-center gap-2">
        {/* View toggle — hidden on mobile, always list on small screens */}
        <div className="hidden sm:flex items-center rounded-md border border-border overflow-hidden">
          <button
            className={`p-1.5 transition-colors ${viewMode === 'grid' ? 'bg-primary text-primary-foreground' : 'hover:bg-muted text-muted-foreground'}`}
            onClick={() => setViewMode('grid')}
            aria-label="Grid view"
            aria-pressed={viewMode === 'grid'}
          >
            <LayoutGrid className="h-4 w-4" aria-hidden="true" />
          </button>
          <button
            className={`p-1.5 transition-colors border-x border-border ${viewMode === 'list' ? 'bg-primary text-primary-foreground' : 'hover:bg-muted text-muted-foreground'}`}
            onClick={() => setViewMode('list')}
            aria-label="List view"
            aria-pressed={viewMode === 'list'}
          >
            <List className="h-4 w-4" aria-hidden="true" />
          </button>
          <button
            className={`p-1.5 transition-colors ${viewMode === 'compact' ? 'bg-primary text-primary-foreground' : 'hover:bg-muted text-muted-foreground'}`}
            onClick={() => setViewMode('compact')}
            aria-label="Compact view"
            aria-pressed={viewMode === 'compact'}
          >
            <AlignJustify className="h-4 w-4" aria-hidden="true" />
          </button>
        </div>

        {/* Sort dropdown */}
        <div className="relative">
          <button
            aria-expanded={sortOpen}
            aria-haspopup="menu"
            className="flex items-center gap-1.5 rounded-md border border-border px-3 py-1.5 text-sm hover:bg-muted transition-colors"
            onClick={() => { setSortOpen((o) => !o); setProjectOpen(false); }}
          >
            Sort: {sortLabels[sortMode]}
            <ChevronDown className="h-3.5 w-3.5" aria-hidden="true" />
          </button>
          {sortOpen && (
            <div role="menu" aria-label="Sort options" className="absolute left-0 top-9 min-w-[140px] rounded-md border border-border bg-popover shadow-md z-50 py-1">
              {(['newest', 'oldest', 'title'] as SortMode[]).map((s) => (
                <button
                  key={s}
                  role="menuitem"
                  className={`w-full text-left px-3 py-1.5 text-sm hover:bg-muted transition-colors ${sortMode === s ? 'font-semibold' : ''}`}
                  onClick={() => { setSortMode(s); setSortOpen(false); }}
                >
                  {sortLabels[s]}
                </button>
              ))}
            </div>
          )}
        </div>

        {/* Project filter dropdown */}
        {projectPaths.length > 1 && (
          <div className="relative">
            <button
              aria-expanded={projectOpen}
              aria-haspopup="menu"
              className="flex items-center gap-1.5 rounded-md border border-border px-3 py-1.5 text-sm hover:bg-muted transition-colors"
              onClick={() => { setProjectOpen((o) => !o); setSortOpen(false); }}
            >
              <FolderOpen className="h-3.5 w-3.5" aria-hidden="true" />
              {projectFilter === 'all' ? 'All Projects' : getProjectFolderName(projectFilter)}
              <ChevronDown className="h-3.5 w-3.5" aria-hidden="true" />
            </button>
            {projectOpen && (
              <div role="menu" aria-label="Filter by project" className="absolute left-0 top-9 min-w-[180px] rounded-md border border-border bg-popover shadow-md z-50 py-1">
                <button
                  role="menuitem"
                  className={`w-full text-left px-3 py-1.5 text-sm hover:bg-muted transition-colors ${projectFilter === 'all' ? 'font-semibold' : ''}`}
                  onClick={() => { setProjectFilter('all'); setProjectOpen(false); }}
                >
                  All Projects
                </button>
                {projectPaths.map((p) => (
                  <button
                    key={p}
                    role="menuitem"
                    className={`w-full text-left px-3 py-1.5 text-sm hover:bg-muted transition-colors truncate ${projectFilter === p ? 'font-semibold' : ''}`}
                    onClick={() => { setProjectFilter(p); setProjectOpen(false); }}
                    title={p}
                  >
                    {getProjectFolderName(p)}
                  </button>
                ))}
              </div>
            )}
          </div>
        )}

        <span className="ml-auto text-xs text-muted-foreground">{filteredSorted.length} task{filteredSorted.length !== 1 ? 's' : ''}</span>
      </div>

      {/* Task list */}
      {filteredSorted.length === 0 ? (
        <EmptyState stageParam={stageParam} onNewTask={onNewTask} />
      ) : viewMode === 'grid' ? (
        <div className="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-3">
          {filteredSorted.map((task) => (
            <TaskCard key={task.id} task={task} />
          ))}
        </div>
      ) : viewMode === 'list' ? (
        <div className="flex flex-col gap-2">
          {filteredSorted.map((task) => (
            <ListRow key={task.id} task={task} />
          ))}
        </div>
      ) : (
        <div className="rounded-md border border-border overflow-hidden">
          {filteredSorted.map((task) => (
            <CompactRow key={task.id} task={task} />
          ))}
        </div>
      )}
    </div>
  );
}
