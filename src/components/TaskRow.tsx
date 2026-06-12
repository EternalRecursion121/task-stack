import { useState } from "react";
import { useSortable } from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import type { Task } from "../types";

const DOT: Record<string, string> = {
  active: "bg-emerald-400",
  "waiting-me": "bg-amber-400 ts-pulse",
  "waiting-machine": "bg-sky-400",
  done: "bg-white/25",
};

function dotKey(t: Task): string {
  if (t.state === "waiting") return t.waiting_kind === "machine" ? "waiting-machine" : "waiting-me";
  return t.state;
}

interface Props {
  task: Task;
  onJump: (t: Task) => void;
  onCycle: (t: Task) => void;
  onComplete: (t: Task) => void;
  onReopen: (t: Task) => void;
  onDelete: (t: Task) => void;
  onBind: (t: Task) => void;
  onRename: (t: Task, title: string) => void;
}

export default function TaskRow({
  task,
  onJump,
  onCycle,
  onComplete,
  onReopen,
  onDelete,
  onBind,
  onRename,
}: Props) {
  const { attributes, listeners, setNodeRef, transform, transition, isDragging } = useSortable({
    id: task.id,
  });
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState(task.title);

  const style = {
    transform: CSS.Transform.toString(transform),
    transition,
    opacity: isDragging ? 0.5 : 1,
  };

  const hasJump = !!task.jump_type && !!task.jump_value;
  const done = task.state === "done";

  const commitRename = () => {
    const next = draft.trim();
    if (next && next !== task.title) onRename(task, next);
    else setDraft(task.title);
    setEditing(false);
  };

  return (
    <div
      ref={setNodeRef}
      style={style}
      {...attributes}
      {...listeners}
      onClick={() => {
        if (!editing && hasJump) onJump(task);
      }}
      className={`group flex items-center gap-2 rounded-lg px-2 py-1.5 transition-colors
        ${hasJump ? "cursor-pointer" : "cursor-default"}
        hover:bg-white/[0.06]`}
    >
      <span className={`h-2 w-2 shrink-0 rounded-full ${DOT[dotKey(task)]}`} />

      <div className="min-w-0 flex-1">
        {editing ? (
          <input
            autoFocus
            value={draft}
            onChange={(e) => setDraft(e.target.value)}
            onBlur={commitRename}
            onKeyDown={(e) => {
              if (e.key === "Enter") commitRename();
              if (e.key === "Escape") {
                setDraft(task.title);
                setEditing(false);
              }
            }}
            onClick={(e) => e.stopPropagation()}
            className="w-full rounded bg-white/10 px-1 py-0.5 text-[13px] outline-none"
          />
        ) : (
          <div className="flex items-center gap-1.5">
            <span
              onDoubleClick={(e) => {
                e.stopPropagation();
                setDraft(task.title);
                setEditing(true);
              }}
              className={`truncate ${done ? "text-white/40 line-through" : "text-white/90"}`}
            >
              {task.title}
            </span>
            {task.project && (
              <span className="shrink-0 rounded bg-white/10 px-1 text-[10px] text-white/55">
                #{task.project}
              </span>
            )}
          </div>
        )}
        {hasJump && !editing && (
          <div className="truncate text-[10px] text-white/35">→ {task.jump_value}</div>
        )}
      </div>

      {/* hover actions */}
      <div className="flex shrink-0 items-center gap-0.5 opacity-0 transition-opacity group-hover:opacity-100">
        {!done && (
          <>
            <IconBtn title="Cycle state" onClick={() => onCycle(task)}>
              ◑
            </IconBtn>
            <IconBtn title="Bind current workspace" onClick={() => onBind(task)}>
              ◎
            </IconBtn>
            <IconBtn title="Complete" onClick={() => onComplete(task)}>
              ✓
            </IconBtn>
          </>
        )}
        {done && (
          <IconBtn title="Reopen" onClick={() => onReopen(task)}>
            ↺
          </IconBtn>
        )}
        <IconBtn title="Delete" onClick={() => onDelete(task)}>
          ✕
        </IconBtn>
      </div>
    </div>
  );
}

function IconBtn({
  children,
  title,
  onClick,
}: {
  children: React.ReactNode;
  title: string;
  onClick: () => void;
}) {
  return (
    <button
      title={title}
      onClick={(e) => {
        e.stopPropagation();
        onClick();
      }}
      onPointerDown={(e) => e.stopPropagation()}
      className="flex h-5 w-5 items-center justify-center rounded text-[11px] text-white/55 hover:bg-white/15 hover:text-white"
    >
      {children}
    </button>
  );
}
