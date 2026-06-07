import type { CSSProperties, ReactNode } from "react";
import type { LayoutBlock, ModuleId } from "../types";

interface ModuleCardProps {
  id: ModuleId;
  title: string;
  icon: string;
  block: LayoutBlock;
  dragging: boolean;
  dropTarget: boolean;
  children: ReactNode;
  headerActions?: ReactNode;
  onCycleSize: (id: ModuleId) => void;
  onResizeStart: (event: React.PointerEvent<HTMLButtonElement>, id: ModuleId) => void;
}

export function ModuleCard({
  id,
  title,
  icon,
  block,
  dragging,
  dropTarget,
  children,
  headerActions,
  onCycleSize,
  onResizeStart,
}: ModuleCardProps) {
  const style = {
    "--cols": block.cols,
    "--rows": block.rows,
    "--min-h": `${block.rows <= 1 ? 194 : 194 + (block.rows - 1) * 206}px`,
  } as CSSProperties;

  return (
    <article
      className={[
        "module",
        dragging ? "dragging" : "",
        dropTarget ? "drop-before" : "",
      ].join(" ")}
      data-module={id}
      draggable
      style={style}
    >
      <header className="module-header">
        <div className="module-title">
          <span className="module-icon" aria-hidden="true">
            {icon}
          </span>
          <span>{title}</span>
        </div>
        <div className="module-tools">
          {headerActions}
          <button className="icon-btn drag-handle" type="button" title="Перетащить">
            <span aria-hidden="true">::</span>
          </button>
          <button
            className="icon-btn"
            type="button"
            title="Быстрый размер"
            onClick={() => onCycleSize(id)}
          >
            <span aria-hidden="true">↗</span>
          </button>
        </div>
      </header>
      <div className="module-content">{children}</div>
      <button
        className="resize-grip"
        type="button"
        title="Потянуть размер"
        aria-label={`Потянуть размер блока ${title}`}
        onPointerDown={(event) => onResizeStart(event, id)}
      />
    </article>
  );
}
