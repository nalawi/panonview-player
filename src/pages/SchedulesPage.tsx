import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import {
  createSchedule,
  deleteSchedule,
  listPages,
  listSchedules,
  updateSchedule,
} from "../services/api";
import type { Day, NewSchedule, Page, Schedule, ScheduleType } from "../types";
import { DAYS } from "../types";

interface FormState {
  page_id: number | "";
  schedule_type: ScheduleType;
  start_time: string;
  end_time: string;
  start_date: string;
  end_date: string;
  days: Day[];
  priority: number;
  enabled: boolean;
  sequence: number | "";
}

const emptyForm: FormState = {
  page_id: "",
  schedule_type: "TIME",
  start_time: "08:00",
  end_time: "12:00",
  start_date: "",
  end_date: "",
  days: [...DAYS],
  priority: 0,
  enabled: true,
  sequence: "",
};

export function SchedulesPage() {
  const qc = useQueryClient();
  const { data: schedules = [] } = useQuery<Schedule[]>({
    queryKey: ["schedules"],
    queryFn: listSchedules,
  });
  const { data: pages = [] } = useQuery<Page[]>({
    queryKey: ["pages"],
    queryFn: listPages,
  });

  const [form, setForm] = useState<FormState>(emptyForm);
  const [editingId, setEditingId] = useState<number | null>(null);
  const [error, setError] = useState<string | null>(null);

  const pageName = (id: number) =>
    pages.find((p) => p.id === id)?.name ?? `#${id}`;

  const invalidate = () => {
    qc.invalidateQueries({ queryKey: ["schedules"] });
    qc.invalidateQueries({ queryKey: ["display-status"] });
  };

  const createMut = useMutation({
    mutationFn: (s: NewSchedule) => createSchedule(s),
    onSuccess: () => {
      setForm(emptyForm);
      invalidate();
    },
    onError: (e) => setError(String(e)),
  });

  const updateMut = useMutation({
    mutationFn: ({ id, s }: { id: number; s: NewSchedule }) =>
      updateSchedule(id, s),
    onSuccess: () => {
      setForm(emptyForm);
      setEditingId(null);
      invalidate();
    },
    onError: (e) => setError(String(e)),
  });

  const deleteMut = useMutation({
    mutationFn: (id: number) => deleteSchedule(id),
    onSuccess: invalidate,
    onError: (e) => setError(String(e)),
  });

  const toPayload = (): NewSchedule | null => {
    if (form.page_id === "") {
      setError("Select a page");
      return null;
    }
    const base: NewSchedule = {
      page_id: Number(form.page_id),
      schedule_type: form.schedule_type,
      priority: form.priority,
      enabled: form.enabled,
    };
    if (form.schedule_type === "TIME") {
      return {
        ...base,
        start_time: form.start_time || null,
        end_time: form.end_time || null,
        start_date: form.start_date || null,
        end_date: form.end_date || null,
        days: form.days.length ? form.days : null,
      };
    }
    return {
      ...base,
      sequence: form.sequence === "" ? null : Number(form.sequence),
    };
  };

  const submit = () => {
    setError(null);
    const payload = toPayload();
    if (!payload) return;
    if (editingId != null) {
      updateMut.mutate({ id: editingId, s: payload });
    } else {
      createMut.mutate(payload);
    }
  };

  const startEdit = (s: Schedule) => {
    setEditingId(s.id);
    setForm({
      page_id: s.page_id,
      schedule_type: s.schedule_type,
      start_time: s.start_time ?? "08:00",
      end_time: s.end_time ?? "12:00",
      start_date: s.start_date ?? "",
      end_date: s.end_date ?? "",
      days: s.days ? (s.days.split(",").map((d) => d.trim()) as Day[]) : [],
      priority: s.priority,
      enabled: s.enabled,
      sequence: s.sequence ?? "",
    });
  };

  const toggleDay = (day: Day) => {
    setForm((f) => ({
      ...f,
      days: f.days.includes(day)
        ? f.days.filter((d) => d !== day)
        : [...f.days, day],
    }));
  };

  return (
    <div className="space-y-6">
      <div className="rounded-lg border border-slate-800 bg-panel p-4">
        <div className="mb-3 text-sm font-medium text-slate-300">
          {editingId != null ? `Edit Schedule #${editingId}` : "Add Schedule"}
        </div>

        <div className="grid grid-cols-12 gap-3">
          <select
            value={form.page_id}
            onChange={(e) =>
              setForm({
                ...form,
                page_id: e.target.value === "" ? "" : Number(e.target.value),
              })
            }
            className="col-span-3 rounded-md border border-slate-700 bg-panel-2 px-3 py-2 text-sm"
          >
            <option value="">Select page…</option>
            {pages.map((p) => (
              <option key={p.id} value={p.id}>
                {p.name}
              </option>
            ))}
          </select>

          <select
            value={form.schedule_type}
            onChange={(e) =>
              setForm({
                ...form,
                schedule_type: e.target.value as ScheduleType,
              })
            }
            className="col-span-2 rounded-md border border-slate-700 bg-panel-2 px-3 py-2 text-sm"
          >
            <option value="TIME">Time-based</option>
            <option value="ROTATION">Rotation</option>
          </select>

          <input
            type="number"
            value={form.priority}
            onChange={(e) =>
              setForm({ ...form, priority: Number(e.target.value) })
            }
            className="col-span-1 rounded-md border border-slate-700 bg-panel-2 px-3 py-2 text-sm"
            title="Priority (higher wins)"
          />

          <label className="col-span-2 flex items-center gap-2 text-sm text-slate-300">
            <input
              type="checkbox"
              checked={form.enabled}
              onChange={(e) => setForm({ ...form, enabled: e.target.checked })}
            />
            Enabled
          </label>

          <button
            onClick={submit}
            className="col-span-2 rounded-md bg-accent px-3 py-2 text-sm font-medium text-white hover:brightness-110"
          >
            {editingId != null ? "Save" : "Add"}
          </button>
        </div>

        {form.schedule_type === "TIME" ? (
          <div className="mt-3 space-y-3">
            <div className="flex flex-wrap items-center gap-3 text-sm">
              <label className="flex items-center gap-2">
                Start
                <input
                  type="time"
                  value={form.start_time}
                  onChange={(e) =>
                    setForm({ ...form, start_time: e.target.value })
                  }
                  className="rounded-md border border-slate-700 bg-panel-2 px-2 py-1"
                />
              </label>
              <label className="flex items-center gap-2">
                End
                <input
                  type="time"
                  value={form.end_time}
                  onChange={(e) => setForm({ ...form, end_time: e.target.value })}
                  className="rounded-md border border-slate-700 bg-panel-2 px-2 py-1"
                />
              </label>
              <label className="flex items-center gap-2">
                Start date
                <input
                  type="date"
                  value={form.start_date}
                  onChange={(e) =>
                    setForm({ ...form, start_date: e.target.value })
                  }
                  className="rounded-md border border-slate-700 bg-panel-2 px-2 py-1"
                />
              </label>
              <label className="flex items-center gap-2">
                End date
                <input
                  type="date"
                  value={form.end_date}
                  onChange={(e) =>
                    setForm({ ...form, end_date: e.target.value })
                  }
                  className="rounded-md border border-slate-700 bg-panel-2 px-2 py-1"
                />
              </label>
            </div>
            <div className="flex flex-wrap gap-2">
              {DAYS.map((d) => (
                <button
                  key={d}
                  onClick={() => toggleDay(d)}
                  className={`rounded-md px-3 py-1 text-xs font-medium ${
                    form.days.includes(d)
                      ? "bg-accent text-white"
                      : "border border-slate-700 text-slate-400 hover:bg-slate-800"
                  }`}
                >
                  {d}
                </button>
              ))}
            </div>
          </div>
        ) : (
          <div className="mt-3 flex items-center gap-3 text-sm">
            <label className="flex items-center gap-2">
              Sequence
              <input
                type="number"
                value={form.sequence}
                onChange={(e) =>
                  setForm({
                    ...form,
                    sequence: e.target.value === "" ? "" : Number(e.target.value),
                  })
                }
                className="w-24 rounded-md border border-slate-700 bg-panel-2 px-2 py-1"
                title="Order within the rotation loop"
              />
            </label>
            <span className="text-xs text-slate-500">
              Pages rotate in sequence order using each page's duration.
            </span>
          </div>
        )}

        {editingId != null && (
          <button
            onClick={() => {
              setEditingId(null);
              setForm(emptyForm);
            }}
            className="mt-3 text-xs text-slate-400 underline"
          >
            Cancel edit
          </button>
        )}
        {error && <div className="mt-2 text-sm text-red-400">{error}</div>}
      </div>

      <div className="overflow-hidden rounded-lg border border-slate-800">
        <table className="w-full text-left text-sm">
          <thead className="bg-panel-2 text-xs uppercase tracking-wide text-slate-400">
            <tr>
              <th className="px-4 py-2">#</th>
              <th className="px-4 py-2">Page</th>
              <th className="px-4 py-2">Type</th>
              <th className="px-4 py-2">Window / Sequence</th>
              <th className="px-4 py-2">Priority</th>
              <th className="px-4 py-2">Enabled</th>
              <th className="px-4 py-2 text-right">Actions</th>
            </tr>
          </thead>
          <tbody className="divide-y divide-slate-800">
            {schedules.map((s) => (
              <tr key={s.id} className="hover:bg-slate-900/40">
                <td className="px-4 py-2 text-slate-500">{s.id}</td>
                <td className="px-4 py-2 font-medium text-slate-200">
                  {pageName(s.page_id)}
                </td>
                <td className="px-4 py-2">
                  <span className="rounded bg-slate-700 px-2 py-0.5 text-xs">
                    {s.schedule_type}
                  </span>
                </td>
                <td className="px-4 py-2 text-slate-400">
                  {s.schedule_type === "TIME"
                    ? `${s.start_time ?? "—"}–${s.end_time ?? "—"}${
                        s.days ? ` · ${s.days}` : ""
                      }`
                    : `seq ${s.sequence ?? "—"}`}
                </td>
                <td className="px-4 py-2 text-slate-400">{s.priority}</td>
                <td className="px-4 py-2">
                  <span
                    className={`rounded px-2 py-0.5 text-xs ${
                      s.enabled
                        ? "bg-emerald-500/15 text-emerald-300"
                        : "bg-slate-700 text-slate-400"
                    }`}
                  >
                    {s.enabled ? "yes" : "no"}
                  </span>
                </td>
                <td className="px-4 py-2 text-right">
                  <button
                    onClick={() => startEdit(s)}
                    className="mr-2 rounded border border-slate-700 px-2 py-1 text-xs hover:bg-slate-800"
                  >
                    Edit
                  </button>
                  <button
                    onClick={() => {
                      if (confirm(`Delete schedule #${s.id}?`)) {
                        deleteMut.mutate(s.id);
                      }
                    }}
                    className="rounded border border-red-900 px-2 py-1 text-xs text-red-400 hover:bg-red-950"
                  >
                    Delete
                  </button>
                </td>
              </tr>
            ))}
            {schedules.length === 0 && (
              <tr>
                <td colSpan={7} className="px-4 py-6 text-center text-slate-500">
                  No schedules yet
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </div>
    </div>
  );
}