'use client';

import { useState, useEffect } from 'react';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { Textarea } from '@/components/ui/textarea';
import { useCreateTask } from '@/hooks/use-tasks';
import { getProjects } from '@/lib/api-client';

interface CreateTaskDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

function sanitizeProjectName(raw: string): string {
  // Remove path separators and collapse .. sequences
  return raw.replace(/[/\\]/g, '').replace(/\.\./g, '');
}

export function CreateTaskDialog({ open, onOpenChange }: CreateTaskDialogProps) {
  const [title, setTitle] = useState('');
  const [description, setDescription] = useState('');
  const [projectName, setProjectName] = useState('');
  const [existingProjects, setExistingProjects] = useState<string[]>([]);

  const createTask = useCreateTask();

  // Fetch existing project names when dialog opens
  useEffect(() => {
    if (open) {
      getProjects().then(setExistingProjects);
    }
  }, [open]);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!title.trim() || !projectName.trim()) return;

    const sanitized = sanitizeProjectName(projectName.trim());
    if (!sanitized) return;

    await createTask.mutateAsync({
      title: title.trim(),
      description: description.trim() || undefined,
      project_path: `~/Projects/${sanitized}`,
    });

    setTitle('');
    setDescription('');
    setProjectName('');
    onOpenChange(false);
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-[425px]">
        <form onSubmit={handleSubmit}>
          <DialogHeader>
            <DialogTitle>Create New Task</DialogTitle>
            <DialogDescription>
              Add a new task to your Kanban board.
            </DialogDescription>
          </DialogHeader>
          <div className="grid gap-4 py-4">
            <div className="grid gap-2">
              <label htmlFor="title" className="text-sm font-medium">
                Title *
              </label>
              <Input
                id="title"
                value={title}
                onChange={(e) => setTitle(e.target.value)}
                placeholder="Enter task title"
                required
              />
            </div>
            <div className="grid gap-2">
              <label htmlFor="description" className="text-sm font-medium">
                Description
              </label>
              <Textarea
                id="description"
                value={description}
                onChange={(e) => setDescription(e.target.value)}
                placeholder="Enter task description (optional)"
                rows={3}
              />
            </div>
            <div className="grid gap-2">
              <label htmlFor="projectName" className="text-sm font-medium">
                Project *
              </label>
              <Input
                id="projectName"
                list="projects-datalist"
                value={projectName}
                onChange={(e) => setProjectName(sanitizeProjectName(e.target.value))}
                placeholder="e.g. my-app"
                required
              />
              <datalist id="projects-datalist">
                {existingProjects.map((p) => (
                  <option key={p} value={p} />
                ))}
              </datalist>
              <p className="text-xs text-muted-foreground">
                Select an existing project or type a new name.
                New directories are created automatically under{' '}
                <code className="font-mono bg-muted px-1 rounded">~/Projects/</code>.
              </p>
            </div>
          </div>
          <DialogFooter>
            <Button
              type="button"
              variant="outline"
              onClick={() => onOpenChange(false)}
            >
              Cancel
            </Button>
            <Button
              type="submit"
              disabled={!title.trim() || !projectName.trim() || createTask.isPending}
            >
              {createTask.isPending ? 'Creating...' : 'Create Task'}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
