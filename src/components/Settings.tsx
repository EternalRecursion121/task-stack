import { useEffect, useState } from "react";
import { api } from "../api";
import type { AeroStatus, Settings as SettingsT } from "../types";

interface Props {
  settings: SettingsT;
  aero: AeroStatus;
  onPatch: (patch: Partial<SettingsT>) => void;
  onAeroRefresh: () => void;
  onClose: () => void;
}

const CORNERS = [
  ["top-left", "↖"],
  ["top-right", "↗"],
  ["bottom-left", "↙"],
  ["bottom-right", "↘"],
] as const;

export default function Settings({ settings, aero, onPatch, onAeroRefresh, onClose }: Props) {
  const [autostart, setAutostart] = useState(false);
  const [hotkey, setHotkey] = useState(settings.hotkey);
  const [captureHotkey, setCaptureHotkey] = useState(settings.capture_hotkey);
  const [captureWsHotkey, setCaptureWsHotkey] = useState(settings.capture_ws_hotkey);

  useEffect(() => {
    api.getAutostart().then(setAutostart).catch(() => {});
  }, []);

  return (
    <div className="px-3 pb-3">
      <div className="mb-2 flex items-center justify-between">
        <span className="text-[11px] font-medium uppercase tracking-wide text-white/45">
          Settings
        </span>
        <button
          onClick={onClose}
          className="rounded px-1.5 text-white/55 hover:bg-white/10 hover:text-white"
        >
          done
        </button>
      </div>

      <Field label="Corner">
        <div className="flex gap-1">
          {CORNERS.map(([key, glyph]) => (
            <button
              key={key}
              onClick={() => {
                api.setCorner(key);
                onPatch({ corner: key });
              }}
              className={`flex h-7 w-7 items-center justify-center rounded-md text-sm
                ${settings.corner === key ? "bg-white/20 text-white" : "bg-white/[0.06] text-white/55 hover:bg-white/10"}`}
            >
              {glyph}
            </button>
          ))}
        </div>
      </Field>

      <Field label="Global hotkey">
        <input
          value={hotkey}
          onChange={(e) => setHotkey(e.target.value)}
          onBlur={() => {
            if (hotkey.trim()) {
              api.setHotkey(hotkey.trim());
              onPatch({ hotkey: hotkey.trim() });
            }
          }}
          spellCheck={false}
          className="w-40 rounded bg-white/[0.06] px-2 py-1 text-right text-[12px] outline-none focus:bg-white/10"
        />
      </Field>

      <Field label="Capture scene (all monitors)">
        <input
          value={captureHotkey}
          onChange={(e) => setCaptureHotkey(e.target.value)}
          onBlur={() => {
            const v = captureHotkey.trim();
            api.setCaptureHotkey(v);
            onPatch({ capture_hotkey: v });
          }}
          spellCheck={false}
          placeholder="off"
          className="w-40 rounded bg-white/[0.06] px-2 py-1 text-right text-[12px] outline-none focus:bg-white/10 placeholder:text-white/30"
        />
      </Field>

      <Field label="Capture workspace (focused)">
        <input
          value={captureWsHotkey}
          onChange={(e) => setCaptureWsHotkey(e.target.value)}
          onBlur={() => {
            const v = captureWsHotkey.trim();
            api.setCaptureWsHotkey(v);
            onPatch({ capture_ws_hotkey: v });
          }}
          spellCheck={false}
          placeholder="off"
          className="w-40 rounded bg-white/[0.06] px-2 py-1 text-right text-[12px] outline-none focus:bg-white/10 placeholder:text-white/30"
        />
      </Field>

      <Field label="Click jumps by">
        <Toggle
          options={[
            ["workspace", "focus"],
            ["summon", "pull here"],
          ]}
          value={settings.jump_mode}
          onChange={(v) => {
            api.setSetting("jump_mode", v);
            onPatch({ jump_mode: v });
          }}
        />
      </Field>

      <Field label="Auto-collapse on blur">
        <Switch
          on={settings.auto_collapse}
          onChange={(on) => {
            api.setSetting("auto_collapse", String(on));
            onPatch({ auto_collapse: on });
          }}
        />
      </Field>

      <Field label="Launch at login">
        <Switch
          on={autostart}
          onChange={(on) => {
            api.setAutostart(on).then(() => setAutostart(on)).catch(() => {});
          }}
        />
      </Field>

      <div className="mt-3 rounded-md bg-white/[0.04] p-2 text-[11px]">
        <div className="mb-1 flex items-center gap-1.5">
          <span
            className={`h-1.5 w-1.5 rounded-full ${
              aero.server_enabled ? "bg-emerald-400" : "bg-amber-400"
            }`}
          />
          <span className="text-white/70">
            AeroSpace{" "}
            {!aero.installed
              ? "not installed"
              : aero.server_enabled
                ? "connected"
                : "server disabled"}
          </span>
        </div>
        {aero.installed && !aero.server_enabled && (
          <button
            onClick={() => api.aerospaceEnable().then(onAeroRefresh)}
            className="rounded bg-white/10 px-2 py-0.5 text-white/80 hover:bg-white/20"
          >
            Enable server
          </button>
        )}
        {aero.message && <div className="mt-1 text-white/35">{aero.message}</div>}
      </div>
    </div>
  );
}

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="flex items-center justify-between py-1.5">
      <span className="text-[12px] text-white/65">{label}</span>
      {children}
    </div>
  );
}

function Switch({ on, onChange }: { on: boolean; onChange: (on: boolean) => void }) {
  return (
    <button
      onClick={() => onChange(!on)}
      className={`relative h-5 w-9 rounded-full transition-colors ${on ? "bg-emerald-500/70" : "bg-white/15"}`}
    >
      <span
        className={`absolute top-0.5 h-4 w-4 rounded-full bg-white transition-all ${on ? "left-[18px]" : "left-0.5"}`}
      />
    </button>
  );
}

function Toggle({
  options,
  value,
  onChange,
}: {
  options: readonly (readonly [string, string])[];
  value: string;
  onChange: (v: string) => void;
}) {
  return (
    <div className="flex rounded-md bg-white/[0.06] p-0.5">
      {options.map(([key, label]) => (
        <button
          key={key}
          onClick={() => onChange(key)}
          className={`rounded px-2 py-0.5 text-[11px] ${
            value === key ? "bg-white/20 text-white" : "text-white/55"
          }`}
        >
          {label}
        </button>
      ))}
    </div>
  );
}
