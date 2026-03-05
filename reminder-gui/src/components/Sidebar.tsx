import {
  Clock,
  Calendar,
  CheckCircle2,
  PauseCircle,
  Tag,
  LayoutGrid,
  Plus,
} from "lucide-react";
import { Button } from "@pikoloo/darwin-ui";

interface SidebarProps {
  viewMode: string;
  onViewModeChange: (view: string) => void;
  tags: string[];
  selectedTag: string | null;
  onSelectTag: (tag: string | null) => void;
  onNewReminder: () => void;
  reminderCounts: {
    all: number;
    today: number;
    week: number;
    upcoming: number;
    paused: number;
    completed: number;
  };
}

const views = [
  { id: "all", label: "全部", icon: LayoutGrid },
  { id: "today", label: "今天", icon: Clock },
  { id: "week", label: "本周", icon: Calendar },
  { id: "upcoming", label: "进行中", icon: CheckCircle2 },
  { id: "paused", label: "已暂停", icon: PauseCircle },
  { id: "completed", label: "已完成", icon: CheckCircle2 },
];

export function Sidebar({
  viewMode,
  onViewModeChange,
  tags,
  selectedTag,
  onSelectTag,
  onNewReminder,
  reminderCounts,
}: SidebarProps) {
  return (
    <aside className="sidebar">
      <Button
        variant="primary"
        fullWidth
        leftIcon={<Plus size={16} />}
        onClick={onNewReminder}
      >
        新建提醒
      </Button>

      <nav className="sidebar-section">
        <div className="sidebar-label">视图</div>
        <ul className="sidebar-nav">
          {views.map((view) => {
            const Icon = view.icon;
            const count = reminderCounts[view.id as keyof typeof reminderCounts];
            const isActive = viewMode === view.id && !selectedTag;
            return (
              <li
                key={view.id}
                className={`sidebar-nav-item ${isActive ? "active" : ""}`}
                onClick={() => {
                  onViewModeChange(view.id);
                  onSelectTag(null);
                }}
              >
                <Icon size={16} />
                <span>{view.label}</span>
                {count > 0 && <span className="count">{count}</span>}
              </li>
            );
          })}
        </ul>
      </nav>

      {tags.length > 0 && (
        <div className="sidebar-section">
          <div className="sidebar-label">标签</div>
          <ul className="sidebar-tags">
            {tags.map((tag) => (
              <li
                key={tag}
                className={`sidebar-tag ${selectedTag === tag ? "active" : ""}`}
                onClick={() => onSelectTag(tag === selectedTag ? null : tag)}
              >
                <Tag size={12} />
                <span>{tag}</span>
              </li>
            ))}
          </ul>
        </div>
      )}
    </aside>
  );
}