import type { CSSProperties, ReactNode } from "react";
import type { LucideIcon } from "lucide-react";
import { GripVertical, Maximize2, X } from "lucide-react";
import type { LayoutBlock, ModuleId } from "../types";

interface ModuleCardProps {
  id: ModuleId;
  title: string;
  icon: LucideIcon;
  block: LayoutBlock;
  dragging: boolean;
  dropTarget: boolean;
  children: ReactNode;
  headerActions?: ReactNode;
  onCycleSize: (id: ModuleId) => void;
  onHide: (id: ModuleId) => void;
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
  onHide,
  onResizeStart,
}: ModuleCardProps) {
  const style = {
    "--cols": block.cols,
    "--rows": block.rows,
    "--min-h": `${block.rows <= 1 ? 194 : 194 + (block.rows - 1) * 206}px`,
    viewTransitionName: `module-${id}`,
  } as CSSProperties;
  const Icon = icon;

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
            <Icon size={15} strokeWidth={2.1} />
          </span>
          <span>{title}</span>
        </div>
        <div className="module-tools">
          {headerActions}
          <button className="icon-btn drag-handle" type="button" title="Перетащить">
            <GripVertical size={16} aria-hidden="true" />
          </button>
          <button
            className="icon-btn"
            type="button"
            title="Быстрый размер"
            onClick={() => onCycleSize(id)}
          >
            <Maximize2 size={15} aria-hidden="true" />
          </button>
          <button
            className="icon-btn"
            type="button"
            title="Скрыть блок"
            aria-label={`Скрыть блок ${title}`}
            onClick={() => onHide(id)}
          >
            <X size={15} aria-hidden="true" />
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
