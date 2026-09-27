import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import {
  createPage,
  deletePage,
  listPages,
  showDisplayPage,
  updatePage,
} from "../services/api";
import type { NewPage, Page } from "../types";

const emptyForm: NewPage = { name: "", url: "", duration: 60, enabled: true };

export function PagesPage() {
  const qc = useQueryClient();
  const { data: pages = [], isLoading } = useQuery<Page[]>({
    queryKey: ["pages"],
    queryFn: listPages,
  });

  const [form, setForm] = useState<NewPage>(emptyForm);
  const [editingId, setEditingId] = useState<number | null>(null);
  const [error, setError] = useState<string | null>(null);

  const invalidate = () => {
    qc.invalidateQueries({ queryKey: ["pages"] });
    qc.invalidateQueries({ queryKey: ["display-status"] });
  };

  const createMut = useMutation({
    mutationFn: (p: NewPage) => createPage(p),
    onSuccess: () => {
      setForm(emptyForm);
      invalidate();
    },
    onError: (e) => setError(String(e)),
  });

  const updateMut = useMutation({
    mutationFn: ({ id, page }: { id: number; page: NewPage }) =>
      updatePage(id, page),
    onSuccess: () => {
      setForm(emptyForm);
      setEditingId(null);
      invalidate();
    },
    onError: (e) => setError(String(e)),
  });

  const deleteMut = useMutation({
    mutationFn: (id: number) => deletePage(id),
    onSuccess: invalidate,
    onError: (e) => setError(String(e)),
  });

  const submit = () => {
    setError(null);
    if (!form.name.trim() || !form.url.trim()) {
      setError("Name and URL are required");
      return;
    }
    if (editingId != null) {
      updateMut.mutate({ id: editingId, page: form });
    } else {
      createMut.mutate(form);
    }
  };

  const startEdit = (p: Page) => {
    setEditingId(p.id);
    setForm({
      name: p.name,
      url: p.url,
      duration: p.duration,
      enabled: p.enabled,
    });
  };

  return (
    <div className="space-y-6">
      <div className="rounded-lg border border-slate-800 bg-panel p-4">
        <div className="mb-3 text-sm font-medium text-slate-300">
          {editingId != null ? `Edit Page #${editingId}` : "Add Page"}
        </div>
        <div className="grid grid-cols-12 gap-3">
          <input
            value={form.name}
            onChange={(e) => setForm({ ...form, name: e.target.value })}
            placeholder="Name"
            className="col-span-3 rounded-md border border-slate-700 bg-panel-2 px-3 py-2 text-sm"
          />
          <input
            value={form.url}
            onChange={(e) => setForm({ ...form, url: e.target.value })}
            placeholder="https://…"
            className="col-span-6 rounded-md border border-slate-700 bg-panel-2 px-3 py-2 text-sm"
          />
          <input
            type="number"
            value={form.duration ?? 60}
            min={1}
            onChange={(e) =>
              setForm({ ...form, duration: Number(e.target.value) })
            }
            placeholder="Duration"
            className="col-span-1 rounded-md border border-slate-700 bg-panel-2 px-3 py-2 text-sm"
            title="Duration (seconds)"
          />
          <label className="col-span-1 flex items-center gap-2 text-sm text-slate-300">
            <input
              type="checkbox"
              checked={form.enabled ?? true}
              onChange={(e) => setForm({ ...form, enabled: e.target.checked })}
            />
            On
          </label>
          <button
            onClick={submit}
            className="col-span-1 rounded-md bg-accent px-3 py-2 text-sm font-medium text-white hover:brightness-110"
          >
            {editingId != null ? "Save" : "Add"}
          </button>
        </div>
        {editingId != null && (
          <button
            onClick={() => {
              setEditingId(null);
              setForm(emptyForm);
            }}
            className="mt-2 text-xs text-slate-400 underline"
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
              <th className="px-4 py-2">Name</th>
              <th className="px-4 py-2">URL</th>
              <th className="px-4 py-2">Duration</th>
              <th className="px-4 py-2">Enabled</th>
              <th className="px-4 py-2 text-right">Actions</th>
            </tr>
          </thead>
          <tbody className="divide-y divide-slate-800">
            {isLoading && (
              <tr>
                <td colSpan={6} className="px-4 py-6 text-center text-slate-500">
                  Loading…
                </td>
              </tr>
            )}
            {pages.map((p) => (
              <tr key={p.id} className="hover:bg-slate-900/40">
                <td className="px-4 py-2 text-slate-500">{p.id}</td>
                <td className="px-4 py-2 font-medium text-slate-200">{p.name}</td>
                <td className="max-w-md truncate px-4 py-2 text-slate-400">
                  {p.url}
                </td>
                <td className="px-4 py-2 text-slate-400">{p.duration}s</td>
                <td className="px-4 py-2">
                  <span
                    className={`rounded px-2 py-0.5 text-xs ${
                      p.enabled
                        ? "bg-emerald-500/15 text-emerald-300"
                        : "bg-slate-700 text-slate-400"
                    }`}
                  >
                    {p.enabled ? "yes" : "no"}
                  </span>
                </td>
                <td className="px-4 py-2 text-right">
                  <button
                    onClick={() => showDisplayPage(p.id)}
                    className="mr-2 rounded border border-slate-700 px-2 py-1 text-xs hover:bg-slate-800"
                  >
                    Show
                  </button>
                  <button
                    onClick={() => startEdit(p)}
                    className="mr-2 rounded border border-slate-700 px-2 py-1 text-xs hover:bg-slate-800"
                  >
                    Edit
                  </button>
                  <button
                    onClick={() => {
                      if (confirm(`Delete page "${p.name}"?`)) {
                        deleteMut.mutate(p.id);
                      }
                    }}
                    className="rounded border border-red-900 px-2 py-1 text-xs text-red-400 hover:bg-red-950"
                  >
                    Delete
                  </button>
                </td>
              </tr>
            ))}
            {!isLoading && pages.length === 0 && (
              <tr>
                <td colSpan={6} className="px-4 py-6 text-center text-slate-500">
                  No pages yet
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </div>
    </div>
  );
}