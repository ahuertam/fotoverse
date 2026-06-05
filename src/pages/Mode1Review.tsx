/**
 * Página de revisión del Modo 1 (duplicados exactos).
 *
 * Layout:
 * - Header con totales.
 * - Lista virtualizada (react-virtuoso) de grupos.
 * - Cada grupo se expande para mostrar sus miembros con checkboxes.
 * - Default sugerido: mantener el miembro con btime_ns más antiguo,
 *   marcar el resto como 'delete' (no se persiste hasta que el usuario
 *   hace click en un checkbox o en el bulk).
 * - Bulk: "Marcar para borrar el resto" aplica el default a todos los
 *   miembros del grupo.
 * - Al final: navegación al siguiente modo (Fase 5+) o volver al inicio.
 *
 * Fase 3: las miniaturas son placeholders grises. La Fase 4 las traerá
 * reales desde `get_thumbnail`.
 */

import { useEffect, useMemo, useState } from "react";
import { useNavigate, useParams } from "react-router-dom";
import { Virtuoso } from "react-virtuoso";

import { Thumbnail } from "../components/Thumbnail";
import {
  bulkSetDecision,
  getExactGroups,
  getGroupMembers,
  setDecision,
} from "../lib/tauri";
import type { ExactGroup, ExactGroupMember } from "../lib/types";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function formatBytes(n: number): string {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  if (n < 1024 * 1024 * 1024) return `${(n / (1024 * 1024)).toFixed(1)} MB`;
  return `${(n / (1024 * 1024 * 1024)).toFixed(2)} GB`;
}

function basename(path: string): string {
  const i = Math.max(path.lastIndexOf("/"), path.lastIndexOf("\\"));
  return i >= 0 ? path.slice(i + 1) : path;
}

// ---------------------------------------------------------------------------
// Componente
// ---------------------------------------------------------------------------

function Mode1Review() {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const sessionId = id != null ? Number(id) : null;

  const [groups, setGroups] = useState<ExactGroup[]>([]);
  const [total, setTotal] = useState(0);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const [expanded, setExpanded] = useState<Set<number>>(new Set());
  const [membersByGroup, setMembersByGroup] = useState<
    Map<number, ExactGroupMember[]>
  >(new Map());
  const [loadingMembers, setLoadingMembers] = useState<Set<number>>(new Set());

  // Carga inicial: todos los grupos (límite alto; Fase 5+ puede paginar
  // server-side si hubiera miles de grupos, que en la práctica no suele
  // pasar para el Modo 1).
  useEffect(() => {
    if (sessionId == null) return;
    let cancelled = false;
    void (async () => {
      try {
        setLoading(true);
        const { groups, total } = await getExactGroups(sessionId, 0, 500);
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
        (acc, g) => acc + g.size_bytes * (g.member_count - 1),
        0,
      ),
    [groups],
  );

  async function toggleExpand(group: ExactGroup) {
    const next = new Set(expanded);
    if (next.has(group.id)) {
      next.delete(group.id);
      setExpanded(next);
      return;
    }
    next.add(group.id);
    setExpanded(next);
    if (membersByGroup.has(group.id)) return; // ya cargado

    setLoadingMembers((prev) => new Set(prev).add(group.id));
    try {
      const { members } = await getGroupMembers(group.id, "exact");
      // Cast: el backend devuelve ExactGroupMember[] cuando
      // mode = "exact".
      setMembersByGroup((prev) => {
        const m = new Map(prev);
        m.set(group.id, members as ExactGroupMember[]);
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
      // Actualizar el estado local
      setMembersByGroup((prev) => {
        const m = new Map(prev);
        const list = m.get(groupId);
        if (list) {
          m.set(
            groupId,
            list.map((mem) => (mem.id === fileId ? { ...mem, decision: action } : mem)),
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
    const keepId = list[0].suggested_keep_id;
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

  // Render -----------------------------------------------------------------

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
        <p>Cargando grupos de duplicados…</p>
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
          <h1>Coincidencias exactas</h1>
          <p className="muted">
            No se encontraron duplicados exactos en esta sesión. 🎉
          </p>
        </header>
        <p className="muted">
          (Los modos perceptual y manual pueden encontrar más duplicados.)
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
        <h1>Coincidencias exactas</h1>
        <p className="summary">
          <strong>{total}</strong> {total === 1 ? "grupo" : "grupos"} ·{" "}
          <strong>{formatBytes(totalWasted)}</strong> desperdiciados
        </p>
        <p className="muted">
          Sugerencia: mantener el archivo con fecha más antigua, marcar
          los demás como "borrar".
        </p>
      </header>

      <Virtuoso
        style={{ height: "60vh" }}
        data={groups}
        itemContent={(_index, group) => {
          const isExpanded = expanded.has(group.id);
          const isLoadingMembers = loadingMembers.has(group.id);
          const members = membersByGroup.get(group.id);
          const wasted = group.size_bytes * (group.member_count - 1);
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
                  <strong className="filename">{group.filename}</strong>
                  <span className="ext">.{group.ext}</span>
                  <span className="count">
                    {group.member_count} archivos
                  </span>
                </div>
                <div className="group-wasted">
                  {formatBytes(wasted)} desperdiciados
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
                          const isSuggested = m.id === m.suggested_keep_id;
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
                                  {formatBytes(m.size_bytes)}
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
                                      void handleDecision(group.id, m.id, "keep")
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
                                      void handleDecision(group.id, m.id, "delete")
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
          (La Fase 5 habilitará "Siguiente: duplicados perceptuales")
        </span>
      </footer>
    </main>
  );
}

export default Mode1Review;
