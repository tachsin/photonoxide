"use client";

import { useEffect, useState } from "react";

/**
 * "New since your last visit", kept in this browser alone (localStorage): the newest version
 * of each row seen on the last visit. A row whose newest release differs from it is marked;
 * on a first visit nothing is. No server state, no cookie.
 */

const KEY = "photonoxide-dependencies-seen";

/** @type {{ at: number, versions: Record<string, string> } | null | undefined} */
let previous;

/** The last visit's record, read once, before this visit's is written. */
function lastVisit() {
  if (previous === undefined) {
    try {
      const parsed = JSON.parse(window.localStorage.getItem(KEY) ?? "null");
      previous = parsed && typeof parsed === "object" && parsed.versions ? parsed : null;
    } catch {
      previous = null;
    }
  }
  return previous;
}

/** A row's marker, when its newest release is new since the last visit. */
export function NewMarker({ id, version }) {
  const [isNew, setNew] = useState(false);
  useEffect(() => {
    const seen = lastVisit()?.versions?.[id];
    setNew(Boolean(version && seen && seen !== version));
  }, [id, version]);
  if (!isNew) return null;
  return (
    <span className="proj-tag" data-tone="accent" title={`new since your last visit, ${new Date(lastVisit().at).toLocaleString()}`}>
      new since your last visit
    </span>
  );
}

/** Records this visit's newest versions, for the next one. */
export function RememberVersions({ versions }) {
  useEffect(() => {
    lastVisit();
    try {
      window.localStorage.setItem(KEY, JSON.stringify({ at: Date.now(), versions }));
    } catch {
      // storage off or full: no marker next time
    }
  }, [versions]);
  return null;
}
