import type { ReactNode } from "react";
import { useSortable } from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";

interface Props {
  id: string;
  className: string;
  onContextMenu?: (e: React.MouseEvent) => void;
  children: ReactNode;
}

/** Item arrastável da dock que não é um app: botão de apps, widget ou separador. */
export function SortableItem({ id, className, onContextMenu, children }: Props) {
  const { listeners, setNodeRef, transform, transition, isDragging } = useSortable({ id });
  return (
    <div
      ref={setNodeRef}
      className={`${className} ${isDragging ? "dragging" : ""}`}
      style={{ transform: CSS.Translate.toString(transform), transition }}
      onContextMenu={onContextMenu}
      {...listeners}
    >
      {children}
    </div>
  );
}
