import { useState, useCallback } from "react";
import { ReminderItem } from "./ReminderItem";
import {
  ChevronDown,
  ChevronRight,
  Clock,
  Calendar,
  RotateCcw,
  Archive,
  AlertCircle,
} from "lucide-react";

interface Reminder {
  id: string;
  title: string;
  description: string | null;
  schedule: { OneTime?: string } | { Cron?: string };
  created_at: string;
  next_trigger: string | null;
  completed: boolean;
  paused: boolean;
  tags: string[];
}

interface ReminderListProps {
  groups: { [key: string]: Reminder[] };
  selectedId: string | null;
  onSelect: (reminder: Reminder | null) => void;
  onPause: (id: string) => void;
  onResume: (id: string) => void;
  onDelete: (id: string) => void;
  onTest: (id: string) => void;
  loading?: boolean;
}

type GroupKey = string;

const groupIcons: { [key: string]: typeof Clock } = {
  "Overdue": AlertCircle,
  "Today": Clock,
  "Tomorrow": Calendar,
  "This Week": Calendar,
  "Later": Calendar,
  "No Date": Clock,
  "Recurring": RotateCcw,
};

const groupLabels: { [key: string]: string } = {
  "Overdue": "已过期",
  "Today": "今天",
  "Tomorrow": "明天",
  "This Week": "本周",
  "Later": "稍后",
  "No Date": "无日期",
  "Recurring": "重复提醒",
};

export function ReminderList({
  groups,
  selectedId,
  onSelect,
  onPause,
  onResume,
  onDelete,
  onTest,
  loading,
}: ReminderListProps) {
  const [collapsedGroups, setCollapsedGroups] = useState<Set<GroupKey>>(
    new Set()
  );

  const toggleGroup = useCallback((key: GroupKey) => {
    setCollapsedGroups((prev) => {
      const next = new Set(prev);
      if (next.has(key)) {
        next.delete(key);
      } else {
        next.add(key);
      }
      return next;
    });
  }, []);

  const groupEntries = Object.entries(groups || {}).filter(
    ([, reminders]) => reminders.length > 0
  );

  const totalReminders = groupEntries.reduce(
    (sum, [, reminders]) => sum + reminders.length,
    0
  );

  if (loading) {
    return (
      <div className="empty-state">
        <div className="empty-icon">
          <Clock size={48} />
        </div>
        <p>加载中...</p>
      </div>
    );
  }

  if (totalReminders === 0) {
    return (
      <div className="empty-state">
        <div className="empty-icon">
          <Clock size={48} />
        </div>
        <p>暂无提醒</p>
        <span>点击「新建提醒」或按 Cmd+N 创建</span>
      </div>
    );
  }

  return (
    <div className="reminder-list-container">
      {groupEntries.map(([groupName, reminders]) => {
        const Icon = groupIcons[groupName] || Archive;
        const isCollapsed = collapsedGroups.has(groupName);

        return (
          <div key={groupName} className="reminder-group">
            <button
              className="group-header"
              onClick={() => toggleGroup(groupName)}
            >
              {isCollapsed ? (
                <ChevronRight size={16} />
              ) : (
                <ChevronDown size={16} />
              )}
              <Icon size={14} />
              <span>{groupLabels[groupName] || groupName}</span>
              <span className="group-count">{reminders.length}</span>
            </button>

            {!isCollapsed && (
              <div className="group-items">
                {reminders.map((reminder) => (
                  <ReminderItem
                    key={reminder.id}
                    reminder={reminder}
                    isSelected={selectedId === reminder.id}
                    onClick={() => onSelect(reminder)}
                    onDelete={onDelete}
                    onPause={onPause}
                    onResume={onResume}
                    onTestTrigger={onTest}
                  />
                ))}
              </div>
            )}
          </div>
        );
      })}
    </div>
  );
}