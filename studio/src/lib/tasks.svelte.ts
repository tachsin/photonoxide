// Examples and validation reports running in processes of their own: their output, kept here so
// it survives leaving the page.

import { api } from "./api";
import { toast } from "./app.svelte";

export interface TaskView {
  id: number;
  lines: string[];
  /** Exit code once it ended. */
  code: number | null;
  seconds: number;
  stopped: boolean;
}

export const tasks = $state<Record<string, TaskView>>({});

/** Starts the task `key` with `start`, and follows its output. */
export async function startTask(key: string, start: () => Promise<number>, done?: (t: TaskView) => void) {
  if (tasks[key] && tasks[key].code === null) return;
  let id: number;
  try {
    id = await start();
  } catch (e) {
    toast(String(e), "error");
    return;
  }
  tasks[key] = { id, lines: [], code: null, seconds: 0, stopped: false };
  const follow = async () => {
    const t = tasks[key];
    if (!t || t.id !== id) return;
    try {
      const p = await api.taskOutput(id, t.lines.length);
      t.lines.push(...p.lines);
      t.seconds = p.seconds;
      if (p.code !== null) {
        t.code = p.code;
        done?.(t);
        return;
      }
    } catch (e) {
      t.code = -1;
      t.lines.push(String(e));
      return;
    }
    setTimeout(follow, 200);
  };
  follow();
}

export function stopTask(key: string) {
  const t = tasks[key];
  if (t && t.code === null) {
    t.stopped = true;
    api.stopTask(t.id);
  }
}
