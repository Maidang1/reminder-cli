import { useState, useCallback } from "react";
import {
  Clock,
  RotateCcw,
  Pause,
  Play,
  Trash2,
  Bell,
  CheckCircle2,
  MoreHorizontal,
  Tag,
} from "lucide-react";
import { Button, Badge } from "@pikoloo/darwin-ui";

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

interface ReminderItemProps {
  reminder: Reminder;
  isSelected: boolean;
  onClick: () => void;
  onDelete: (id: string) => void;
  onPause: (id: string) => void;
  onResume: (id: string) => void;
  onTestTrigger: (id: string) => void;
}

export function ReminderItem({
  reminder,
  isSelected,
  onClick,
  onDelete,
  onPause,
  onResume,
  onTestTrigger,
}: ReminderItemProps) {
  const [showActions, setShowActions] = useState(false);
  const [isMenuOpen, setIsMenuOpen] = useState(false);

  const shortId = reminder.id.substring(0, 8);
  const isRecurring = "Cron" in reminder.schedule;
  const scheduleText = isRecurring
    ? (reminder.schedule as { Cron: string }).Cron
    : reminder.next_trigger
      ? formatTime(reminder.next_trigger)
      : "无触发时间";

  const handleAction = useCallback(
    (e: React.MouseEvent, action: () => void) => {
      e.stopPropagation();
      action();
      setIsMenuOpen(false);
    },
    []
  );

  return (
    <div
      className={`reminder-item ${isSelected ? "selected" : ""} ${reminder.paused ? "paused" : ""} ${reminder.completed ? "completed" : ""}`}
      onClick={onClick}
      onMouseEnter={() => setShowActions(true)}
      onMouseLeave={() => {
        setShowActions(false);
        setIsMenuOpen(false);
      }}
    >
      <div className="reminder-icon">
        {reminder.completed ? (
          <CheckCircle2 size={18} className="icon-completed" />
        ) : isRecurring ? (
          <RotateCcw size={18} className="icon-recurring" />
        ) : (
          <Clock size={18} className="icon-onetime" />
        )}
      </div>

      <div className="reminder-content">
        <div className="reminder-title-row">
          <span className="reminder-title">{reminder.title}</span>
          {reminder.paused && (
            <Badge variant="warning" style={{ display: 'inline-flex', alignItems: 'center', gap: '4px', padding: '2px 6px', fontSize: '11px', lineHeight: '1' }}>
              <Pause size={10} style={{ display: 'inline-flex', alignItems: 'center' }} />
              <span>已暂停</span>
            </Badge>
          )}
        </div>
        <div className="reminder-meta-row">
          <span className="reminder-time" style={{ display: 'inline-flex', alignItems: 'center', lineHeight: '1' }}>{scheduleText}</span>
          {reminder.tags.length > 0 && (
            <span className="reminder-tags-preview" style={{ display: 'inline-flex', alignItems: 'center', gap: '4px', lineHeight: '1' }}>
              <Tag size={10} />
              <span>{reminder.tags.slice(0, 2).join(", ")}
              {reminder.tags.length > 2 && ` +${reminder.tags.length - 2}`}</span>
            </span>
          )}
        </div>
      </div>

      <div className={`reminder-actions ${showActions ? "visible" : ""}`}>
        {!reminder.completed && (
          <>
            <Button.Icon variant="ghost" onClick={(e: React.MouseEvent) => handleAction(e, () => onTestTrigger(shortId))}>
              <Bell size={14} />
            </Button.Icon>
            {reminder.paused ? (
              <Button.Icon variant="ghost" onClick={(e: React.MouseEvent) => handleAction(e, () => onResume(shortId))}>
                <Play size={14} />
              </Button.Icon>
            ) : (
              <Button.Icon variant="ghost" onClick={(e: React.MouseEvent) => handleAction(e, () => onPause(shortId))}>
                <Pause size={14} />
              </Button.Icon>
            )}
          </>
        )}

        <div className="action-menu">
          <Button.Icon variant="ghost" onClick={(e: React.MouseEvent) => {
            e.stopPropagation();
            setIsMenuOpen(!isMenuOpen);
          }}>
            <MoreHorizontal size={14} />
          </Button.Icon>

          {isMenuOpen && (
            <div className="action-menu-dropdown">
              <Button
                variant="ghost"
                size="sm"
                leftIcon={<Trash2 size={14} />}
                onClick={(e: React.MouseEvent) =>
                  handleAction(e, () => {
                    if (confirm("确定删除此提醒？")) {
                      onDelete(shortId);
                    }
                  })
                }
              >
                删除
              </Button>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

function formatTime(isoString: string): string {
  const date = new Date(isoString);
  const now = new Date();
  const today = new Date(now.getFullYear(), now.getMonth(), now.getDate());
  const dateDay = new Date(date.getFullYear(), date.getMonth(), date.getDate());

  const hours = date.getHours().toString().padStart(2, "0");
  const minutes = date.getMinutes().toString().padStart(2, "0");

  if (dateDay.getTime() === today.getTime()) {
    return `今天 ${hours}:${minutes}`;
  }

  const tomorrow = new Date(today);
  tomorrow.setDate(tomorrow.getDate() + 1);
  if (dateDay.getTime() === tomorrow.getTime()) {
    return `明天 ${hours}:${minutes}`;
  }

  const month = (date.getMonth() + 1).toString().padStart(2, "0");
  const day = date.getDate().toString().padStart(2, "0");
  return `${month}/${day} ${hours}:${minutes}`;
}