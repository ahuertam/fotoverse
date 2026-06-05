/**
 * Página de revisión del Modo 2 (duplicados perceptuales).
 *
 * Mismo layout que Mode1Review pero con:
 * - Distancia Hamming al representante (badge por miembro)
 * - El "más antiguo" (btime_ns mínimo) se sugiere como `keep` por
 *   defecto, los demás como `delete`.
 *
 * Fase 5: las decisiones usan los mismos comandos `set_decision` /
 * `bulk_set_decision` que Modo 1.
 */

import { useEffect, useMemo, useState } from "react";
import { useNavigate, useParams } from "react-router-dom";
import { Virtuoso } from "react-virtuoso";

import { Thumbnail } from "../components/Thumbnail";
import {
  bulkSetDecision,
  getGroupMembers,
  getPerceptualGroups,
  setDecision,
} from "../lib/tauri";
import type {
  PerceptualGroup,
  PerceptualGroupMember,
} from "../lib/types";

function formatBytes(n: number): string {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  if (n < 1024 * 1024 * 1024)
    return `${(n / (1024 * 1024)).toFixed(1)} MB`;
  return `${(n / (1024 * 1024 * 1024)).toFixed(2)} GB`;
}

function basename(path: string): string {
  const i = Math.max(path.lastIndexOf("/"), path.lastIndexOf("\\"));
  return i >= 0 ? path.slice(i + 1) : path;
}

function Mode2Review() {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const sessionId = id != null ? Number(id) : null;

  const [groups, setGroups] = useState<PerceptualGroup[]>([]);
  const [total, setTotal] = useState(0);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const [expanded, setExpanded] = useState<Set<number>>(new Set());
  const [membersByGroup, setMembersByGroup] = useState<
    Map<number, PerceptualGroupMember[]>
  >(new Map());
  const [loadingMembers, setLoadingMembers] = useState<Set<number>>(
    new Set(),
  );

  useEffect(() => {
    if (sessionId == null) return;
    let cancelled = false;
    void (async () => {
      try {
        setLoading(true);
        const { groups, total } = await getPerceptualGroups(
          sessionId,
          0,
          500,
        );
        if (!cancelled) {
          setGroups(groups);
          setTotal(total);
        }
      } catch (e) {
        if (!cancelled) setError(String(e));
      } finally {
        if (!cancelled) setLoading(false);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [sessionId]);

  const totalWasted = useMemo(
    () =>
      groups.reduce(
        (acc, g) => acc + (g.memberCount - 1), // nº de "copias"
          0,
      ),
    [groups],
  );

  async function toggleExpand(group: PerceptualGroup) {
    const next = new Set(expanded);
    if (next.has(group.id)) {
      next.delete(group.id);
      setExpanded(next);
      return;
    }
    next.add(group.id);
    setExpanded(next);
    if (membersByGroup.has(group.id)) return;

    setLoadingMembers((prev) => new Set(prev).add(group.id));
    try {
      const result = await getGroupMembers(group.id, "perceptual");
      // Cast: el backend devuelve PerceptualGroupMember[] cuando
      // mode = "perceptual".
      setMembersByGroup((prev) => {
        const m = new Map(prev);
        m.set(
          group.id,
          result.members as PerceptualGroupMember[],
        );
        return m;
      });
    } catch (e) {
      setError(String(e));
    } finally {
      setLoadingMembers((prev) => {
        const m = new Set(prev);
        m.delete(group.id);
        return m;
      });
    }
  }

  async function handleDecision(
    groupId: number,
    fileId: number,
    action: "delete" | "keep" | "undecided",
  ) {
    try {
      await setDecision(fileId, action);
      setMembersByGroup((prev) => {
        const m = new Map(prev);
        const list = m.get(groupId);
        if (list) {
          m.set(
            groupId,
            list.map((mem) =>
              mem.id === fileId ? { ...mem, decision: action } : mem,
            ),
          );
        }
        return m;
      });
    } catch (e) {
      setError(String(e));
    }
  }

  async function handleBulkKeepOldest(groupId: number) {
    const list = membersByGroup.get(groupId);
    if (!list || list.length === 0) return;
    // El primer elemento es el representante (btime_ns mínimo). El
    // resto van a "delete".
    const keepId = list[0].id;
    const deleteIds = list.filter((m) => m.id !== keepId).map((m) => m.id);
    try {
      await bulkSetDecision([keepId], "keep");
      await bulkSetDecision(deleteIds, "delete");
      setMembersByGroup((prev) => {
        const m = new Map(prev);
        m.set(
          groupId,
          list.map((mem) => ({
            ...mem,
            decision: mem.id === keepId ? "keep" : "delete",
          })),
        );
        return m;
      });
    } catch (e) {
      setError(String(e));
    }
  }

  if (sessionId == null) {
    return (
      <main className="mode1-review">
        <p>ID de sesión inválido.</p>
        <button type="button" onClick={() => navigate("/")}>
          Volver al inicio
        </button>
      </main>
    );
  }

  if (loading) {
    return (
      <main className="mode1-review">
        <p>Cargando grupos de duplicados perceptuales…</p>
      </main>
    );
  }

  if (error) {
    return (
      <main className="mode1-review">
        <p className="error">⚠️ {error}</p>
        <button type="button" onClick={() => navigate("/")}>
          Volver al inicio
        </button>
      </main>
    );
  }

  if (groups.length === 0) {
    return (
      <main className="mode1-review">
        <header>
          <h1>Coincidencias perceptuales</h1>
          <p className="muted">
            No se encontraron duplicados perceptuales en esta sesión. 🎉
          </p>
        </header>
        <p className="muted">
          (Modos 3 puede encontrar más; sólo aparecerán los que no
          hayas decidido ya.)
        </p>
        <button type="button" onClick={() => navigate("/")}>
          Volver al inicio
        </button>
      </main>
    );
  }

  return (
    <main className="mode1-review">
      <header>
        <h1>Coincidencias perceptuales</h1>
        <p className="summary">
          <strong>{total}</strong> {total === 1 ? "grupo" : "grupos"} ·{" "}
          <strong>{totalWasted}</strong> copias detectadas
        </p>
        <p className="muted">
          Sugerencia: mantener el más antiguo (badge "Más antiguo"),
          borrar los perceptualmente similares.
        </p>
      </header>

      <Virtuoso
        style={{ height: "60vh" }}
        data={groups}
        itemContent={(_index, group) => {
          const isExpanded = expanded.has(group.id);
          const isLoadingMembers = loadingMembers.has(group.id);
          const members = membersByGroup.get(group.id);
          return (
            <div className={`group ${isExpanded ? "expanded" : ""}`}>
              <div
                className="group-summary"
                onClick={() => void toggleExpand(group)}
                role="button"
                tabIndex={0}
                onKeyDown={(e) => {
                  if (e.key === "Enter" || e.key === " ") {
                    e.preventDefault();
                    void toggleExpand(group);
                  }
                }}
              >
                <div className="group-info">
                  <span className="caret">{isExpanded ? "▾" : "▸"}</span>
                  <strong className="filename">
                    Cluster perceptual
                  </strong>
                  <span className="count">
                    {group.memberCount} archivos
                  </span>
                </div>
                <div className="group-wasted">
                  {group.memberCount - 1}{" "}
                  {group.memberCount - 1 === 1 ? "copia" : "copias"}
                </div>
              </div>

              {isExpanded && (
                <div className="group-members">
                  {isLoadingMembers && <p className="muted">Cargando…</p>}
                  {members && (
                    <>
                      <button
                        type="button"
                        className="bulk"
                        onClick={() => void handleBulkKeepOldest(group.id)}
                      >
                        ✨ Mantener el más antiguo, borrar el resto
                      </button>
                      <ul>
                        {members.map((m) => {
                          const decision = m.decision ?? "undecided";
                          const isSuggested = m.id === group.representative;
                          return (
                            <li
                              key={m.id}
                              className={`member ${isSuggested ? "suggested" : ""}`}
                            >
                              <div className="thumb">
                                <Thumbnail
                                  fileId={m.id}
                                  title={m.path}
                                  className="thumb-img"
                                />
                              </div>
                              <div className="meta">
                                <p className="path" title={m.path}>
                                  {basename(m.path)}
                                </p>
                                <p className="path-full">{m.path}</p>
                                <p className="muted small">
                                  {formatBytes(m.sizeBytes)} ·{" "}
                                  <span className="hamming">
                                    Hamming: {m.hammingToRep}
                                  </span>
                                </p>
                                {isSuggested && (
                                  <span className="badge">Más antiguo</span>
                                )}
                              </div>
                              <div className="actions">
                                <label>
                                  <input
                                    type="radio"
                                    name={`decision-${m.id}`}
                                    checked={decision === "keep"}
                                    onChange={() =>
                                      void handleDecision(
                                        group.id,
                                        m.id,
                                        "keep",
                                      )
                                    }
                                  />
                                  Mantener
                                </label>
                                <label>
                                  <input
                                    type="radio"
                                    name={`decision-${m.id}`}
                                    checked={decision === "delete"}
                                    onChange={() =>
                                      void handleDecision(
                                        group.id,
                                        m.id,
                                        "delete",
                                      )
                                    }
                                  />
                                  Borrar
                                </label>
                                <label>
                                  <input
                                    type="radio"
                                    name={`decision-${m.id}`}
                                    checked={decision === "undecided"}
                                    onChange={() =>
                                      void handleDecision(
                                        group.id,
                                        m.id,
                                        "undecided",
                                      )
                                    }
                                  />
                                  Sin decidir
                                </label>
                              </div>
                            </li>
                          );
                        })}
                      </ul>
                    </>
                  )}
                </div>
              )}
            </div>
          );
        }}
      />

      <footer className="mode1-footer">
        <button type="button" onClick={() => navigate("/")}>
          ← Volver al inicio
        </button>
        <span className="muted">
          (Fase 6 habilitará "Siguiente: vídeos perceptuales")
        </span>
      </footer>
    </main>
  );
}

export default Mode2Review;
