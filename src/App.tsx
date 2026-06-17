import { useCallback, useEffect, useLayoutEffect, useRef, useState } from "react";
import {
  DndContext,
  PointerSensor,
  closestCenter,
  useSensor,
  useSensors,
  type DragEndEvent,
} from "@dnd-kit/core";
import { SortableContext, arrayMove, verticalListSortingStrategy } from "@dnd-kit/sortable";
import { listen } from "@tauri-apps/api/event";
import { api } from "./api";
import type { AeroStatus, JumpType, Settings as SettingsT, Task } from "./types";
import TaskRow from "./components/TaskRow";
import Settings from "./components/Settings";

const WIDTH = 340;
const MAX_HEIGHT = 680;

const DEFAULT_SETTINGS: SettingsT = {
  corner: "top-right",
  hotkey: "CmdOrCtrl+Space",
  capture_hotkey: "CmdOrCtrl+Shift+Space",
  capture_ws_hotkey: "CmdOrCtrl+Alt+Space",
  jump_mode: "workspace",
  auto_collapse: false,
};

export default function App() {
  const [tasks, setTasks] = useState<Task[]>([]);
  const [settings, setSettings] = useState<SettingsT>(DEFAULT_SETTINGS);
  const [aero, setAero] = useState<AeroStatus>({
    installed: false,
    server_enabled: false,
    message: null,
  });
  const [collapsed, setCollapsed] = useState(false);
  const [doneOpen, setDoneOpen] = useState(false);
  const [view, setView] = useState<"list" | "settings">("list");
  const [draft, setDraft] = useState("");
  const [bindNext, setBindNext] = useState(false);
  // Task just created by the capture hotkey — it opens straight into rename mode.
  const [captureId, setCaptureId] = useState<string | null>(null);

  const measureRef = useRef<HTMLDivElement>(null);

  // ---- initial load + tray "settings" event ----
  useEffect(() => {
    api.bootstrap().then((b) => {
      setTasks(b.tasks);
      setSettings({
        corner: b.corner,
        hotkey: b.hotkey,
        capture_hotkey: b.capture_hotkey,
        capture_ws_hotkey: b.capture_ws_hotkey,
        jump_mode: b.jump_mode,
        auto_collapse: b.auto_collapse,
      });
      setAero(b.aerospace);
    });
    const un = listen("open-settings", () => {
      setCollapsed(false);
      setView("settings");
    });
    // Native ⌘↑/⌘↓ corner snapping reports the new corner back to keep us in sync.
    const unCorner = listen<string>("corner-changed", (e) => {
      setSettings((s) => ({ ...s, corner: e.payload }));
    });
    // Capture hotkeys: bind the current workspace to a fresh task, opened straight
    // into rename mode. Payload `focusedOnly` picks the single focused workspace vs
    // the full multi-monitor scene (mirrors ◎ vs ⌥-◎).
    const unCapture = listen<boolean>("capture-workspace", async (e) => {
      const focusedOnly = e.payload;
      setCollapsed(false);
      setView("list");
      try {
        let jumpType: JumpType | null;
        let jumpValue: string | null;
        let title: string;
        if (focusedOnly) {
          const ws = await api.aerospaceFocusedWorkspace();
          jumpType = ws ? "workspace" : null;
          jumpValue = ws || null;
          title = ws || "New task";
        } else {
          const scene = await api.aerospaceVisibleScene();
          const hasScene = scene.length > 0;
          jumpType = hasScene ? "scene" : null;
          jumpValue = hasScene ? JSON.stringify(scene) : null;
          title = hasScene ? scene.join(" · ") : "New task";
        }
        const t = await api.createTask(title, null, jumpType, jumpValue);
        setTasks((prev) => [t, ...prev]);
        setCaptureId(t.id);
      } catch (err) {
        console.error("capture failed:", err);
      }
    });
    return () => {
      un.then((f) => f());
      unCorner.then((f) => f());
      unCapture.then((f) => f());
    };
  }, []);

  // ---- auto-collapse on blur ----
  useEffect(() => {
    const onBlur = () => {
      if (settings.auto_collapse) setCollapsed(true);
    };
    window.addEventListener("blur", onBlur);
    return () => window.removeEventListener("blur", onBlur);
  }, [settings.auto_collapse]);

  // ---- resize the OS panel to hug content ----
  useLayoutEffect(() => {
    const el = measureRef.current;
    if (!el) return;
    const apply = () => {
      const rect = el.getBoundingClientRect();
      const w = collapsed ? Math.ceil(rect.width) : WIDTH;
      const h = Math.min(Math.ceil(rect.height), MAX_HEIGHT);
      api.setSize(w, Math.max(h, 36));
    };
    apply();
    const ro = new ResizeObserver(apply);
    ro.observe(el);
    return () => ro.disconnect();
  }, [collapsed, view, tasks, doneOpen]);

  // ---- local mutation helpers ----
  const replace = (t: Task) => setTasks((prev) => prev.map((x) => (x.id === t.id ? t : x)));
  const remove = (id: string) => setTasks((prev) => prev.filter((x) => x.id !== id));

  const patchSettings = (patch: Partial<SettingsT>) =>
    setSettings((s) => ({ ...s, ...patch }));

  // ---- ⌘+arrow snaps the panel between corners (when it's focused) ----
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (!e.metaKey || e.shiftKey || e.altKey || e.ctrlKey) return;
      const tag = (e.target as HTMLElement | null)?.tagName;
      if (tag === "INPUT" || tag === "TEXTAREA") return;
      const [v] = settings.corner.split("-");
      let next = settings.corner;
      if (e.key === "ArrowLeft") next = `${v}-left`;
      else if (e.key === "ArrowRight") next = `${v}-right`;
      // ⌘↑/⌘↓ are handled natively (WKWebView swallows them here) — see lib.rs.
      else return;
      e.preventDefault();
      if (next !== settings.corner) {
        api.setCorner(next);
        patchSettings({ corner: next });
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [settings.corner]);

  // ---- groups ----
  const active = tasks.filter((t) => t.state === "active");
  const done = tasks.filter((t) => t.state === "done");

  // ---- actions ----
  const addTask = useCallback(async () => {
    const raw = draft.trim();
    if (!raw) return;
    const m = raw.match(/#(\S+)/);
    const project = m ? m[1] : null;
    const title = raw.replace(/#\S+/, "").trim() || raw;
    let jumpType: JumpType | null = null;
    let jumpValue: string | null = null;
    if (bindNext) {
      try {
        const scene = await api.aerospaceVisibleScene();
        if (scene.length) {
          jumpValue = JSON.stringify(scene);
          jumpType = "scene";
        }
      } catch {
        /* aerospace unavailable — add without a binding */
      }
    }
    const t = await api.createTask(title, project, jumpType, jumpValue);
    setTasks((prev) => [t, ...prev]);
    setDraft("");
  }, [draft, bindNext]);

  // Persist a new order for a group of tasks, reflecting it locally first.
  const applyOrder = (reordered: Task[]) => {
    const order = new Map(reordered.map((t, i) => [t.id, i]));
    setTasks((prev) =>
      [...prev].sort((x, y) => {
        const ix = order.get(x.id);
        const iy = order.get(y.id);
        if (ix !== undefined && iy !== undefined) return ix - iy;
        return 0;
      }),
    );
    api.reorder(reordered.map((t) => t.id)).catch(() => {});
  };

  // Defer: drop the task to the bottom of the active queue so what you can act
  // on rises to the top. "Waiting" is just position now, not a state.
  const defer = (t: Task) => applyOrder([...active.filter((x) => x.id !== t.id), t]);

  const complete = (t: Task) => api.setState(t.id, "done").then(replace);
  const reopen = (t: Task) => api.setState(t.id, "active").then(replace);
  const del = (t: Task) => api.deleteTask(t.id).then(() => remove(t.id));
  const rename = (t: Task, title: string) =>
    api.updateTitle(t.id, title, t.project).then(replace);
  const jump = (t: Task) => {
    if (t.jump_type && t.jump_value)
      api.jump(t.jump_type, t.jump_value).catch((e) => console.error("jump failed:", e));
  };
  const bind = async (t: Task, focusedOnly = false) => {
    try {
      if (focusedOnly) {
        // Just the focused monitor's workspace — stored as a plain workspace jump.
        const ws = await api.aerospaceFocusedWorkspace();
        const updated = await api.setJump(t.id, "workspace", ws);
        replace(updated);
      } else {
        // The whole arrangement across every monitor.
        const scene = await api.aerospaceVisibleScene();
        if (!scene.length) return;
        const updated = await api.setJump(t.id, "scene", JSON.stringify(scene));
        replace(updated);
      }
    } catch (e) {
      console.error("bind failed:", e);
    }
  };

  const refreshAero = () => api.aerospaceStatus().then(setAero);

  // ---- drag reorder (within a group) ----
  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 6 } }),
  );
  const onDragEnd = (e: DragEndEvent) => {
    const { active: a, over } = e;
    if (!over || a.id === over.id) return;
    const group = [active, done].find((g) => g.some((t) => t.id === a.id));
    if (!group || !group.some((t) => t.id === over.id)) return;
    const oldIndex = group.findIndex((t) => t.id === a.id);
    const newIndex = group.findIndex((t) => t.id === over.id);
    applyOrder(arrayMove(group, oldIndex, newIndex));
  };

  return (
    <div
      ref={measureRef}
      style={{ width: collapsed ? "fit-content" : WIDTH }}
      className="overflow-hidden bg-neutral-900/80 text-white/90"
    >
      {/* header / collapsed pill */}
      <div className={`flex items-center gap-2 px-3 ${collapsed ? "py-1.5 whitespace-nowrap" : "py-2"}`}>
        <button
          onClick={() => setCollapsed((c) => !c)}
          className="shrink-0 text-white/45 hover:text-white"
          title={collapsed ? "Expand" : "Collapse"}
        >
          {collapsed ? "▸" : "▾"}
        </button>
        <div className={`flex items-center gap-2 text-[12px] ${collapsed ? "" : "min-w-0 flex-1"}`}>
          <Count color="bg-emerald-400" n={active.length} label="active" />
        </div>
        {!collapsed && view === "list" && (
          <button
            onClick={() => setView("settings")}
            className="shrink-0 text-white/40 hover:text-white"
            title="Settings"
          >
            ⚙
          </button>
        )}
      </div>

      {!collapsed && view === "settings" && (
        <Settings
          settings={settings}
          aero={aero}
          onPatch={patchSettings}
          onAeroRefresh={refreshAero}
          onClose={() => setView("list")}
        />
      )}

      {!collapsed && view === "list" && (
        <div className="scroll-area max-h-[600px] overflow-y-auto px-2 pb-2">
          {/* quick add */}
          <div className="mb-1 flex items-center gap-1 px-1">
            <input
              value={draft}
              onChange={(e) => setDraft(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && addTask()}
              placeholder="Add a task…  #project"
              className="flex-1 rounded-lg bg-white/[0.06] px-2 py-1.5 text-[13px] outline-none placeholder:text-white/30 focus:bg-white/10"
            />
            <button
              onClick={() => setBindNext((b) => !b)}
              title="Bind new task to current spaces (all monitors)"
              className={`flex h-7 w-7 items-center justify-center rounded-lg text-sm ${
                bindNext ? "bg-emerald-500/40 text-white" : "bg-white/[0.06] text-white/45"
              }`}
            >
              ◎
            </button>
          </div>

          <DndContext sensors={sensors} collisionDetection={closestCenter} onDragEnd={onDragEnd}>
            <Group title="Active" tasks={active} hideEmpty>
              {active.map((t) => row(t))}
            </Group>
            {done.length > 0 && (
              <div className="mt-1">
                <button
                  onClick={() => setDoneOpen((o) => !o)}
                  className="flex w-full items-center gap-1 px-2 py-1 text-[10px] uppercase tracking-wide text-white/35 hover:text-white/60"
                >
                  {doneOpen ? "▾" : "▸"} Done · {done.length}
                </button>
                {doneOpen && (
                  <Group title="" tasks={done}>
                    {done.map((t) => row(t))}
                  </Group>
                )}
              </div>
            )}
            {tasks.length === 0 && (
              <div className="px-3 py-4 text-center text-[12px] text-white/35">
                Nothing yet. Add your first thread above.
              </div>
            )}
          </DndContext>
        </div>
      )}
    </div>
  );

  function row(t: Task) {
    return (
      <TaskRow
        key={t.id}
        task={t}
        autoEdit={t.id === captureId}
        onJump={jump}
        onDefer={defer}
        onComplete={complete}
        onReopen={reopen}
        onDelete={del}
        onBind={bind}
        onRename={rename}
      />
    );
  }
}

function Count({ color, n, label }: { color: string; n: number; label: string }) {
  return (
    <span className="flex items-center gap-1 whitespace-nowrap text-white/60">
      <span className={`h-1.5 w-1.5 rounded-full ${color}`} />
      {n} {label}
    </span>
  );
}

function Group({
  title,
  tasks,
  children,
  hideEmpty,
}: {
  title: string;
  tasks: Task[];
  children: React.ReactNode;
  hideEmpty?: boolean;
}) {
  if (hideEmpty && tasks.length === 0) return null;
  return (
    <div className="mb-1">
      {title && (
        <div className="px-2 pb-0.5 pt-1 text-[10px] font-medium uppercase tracking-wide text-white/35">
          {title}
        </div>
      )}
      <SortableContext items={tasks.map((t) => t.id)} strategy={verticalListSortingStrategy}>
        {children}
      </SortableContext>
    </div>
  );
}
