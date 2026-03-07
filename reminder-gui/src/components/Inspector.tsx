import { useState, useEffect, useCallback } from "react";
import {
  Clock,
  RotateCcw,
  Tag,
  FileText,
  Bell,
  Pause,
  Play,
  Trash2,
  X,
  Save,
  AlertCircle,
} from "lucide-react";
import { Button, Input, Badge } from "@pikoloo/darwin-ui";

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

interface InspectorProps {
  reminder: Reminder | null;
  onClose: () => void;
  onUpdate: (
    id: string,
    data: { title?: string; description?: string; tags?: string[] }
  ) => void;
  onDelete: (id: string) => void;
  onPause: (id: string) => void;
  onResume: (id: string) => void;
  onTestTrigger: (id: string) => void;
}

export function Inspector({
  reminder,
  onClose,
  onUpdate,
  onDelete,
  onPause,
  onResume,
  onTestTrigger,
}: InspectorProps) {
  const [editData, setEditData] = useState({
    title: "",
    description: "",
    tags: "",
  });
  const [hasChanges, setHasChanges] = useState(false);

  useEffect(() => {
    if (reminder) {
      setEditData({
        title: reminder.title,
        description: reminder.description || "",
        tags: reminder.tags.join(", "),
      });
      setHasChanges(false);
    }
  }, [reminder?.id]);

  const handleChange = useCallback(
    (field: keyof typeof editData, value: string) => {
      setEditData((prev) => ({ ...prev, [field]: value }));
      setHasChanges(true);
    },
    []
  );

  const handleSave = useCallback(() => {
    if (!reminder) return;
    const shortId = reminder.id.substring(0, 8);
    onUpdate(shortId, {
      title: editData.title,
      description: editData.description || undefined,
      tags: editData.tags
        .split(",")
        .map((t) => t.trim())
        .filter(Boolean),
    });
    setHasChanges(false);
  }, [reminder, editData, onUpdate]);

  const handleDelete = useCallback(() => {
    if (!reminder) return;
    if (confirm("确定删除此提醒？此操作不可撤销。")) {
      onDelete(reminder.id.substring(0, 8));
      onClose();
    }
  }, [reminder, onDelete, onClose]);

  if (!reminder) {
    return (
      <aside className="inspector empty">
        <div className="inspector-placeholder">
          <Clock size={48} />
          <p>选择一个提醒查看详情</p>
          <span>或按 Cmd+N 创建新提醒</span>
        </div>
      </aside>
    );
  }

  const shortId = reminder.id.substring(0, 8);
  const isRecurring = "Cron" in reminder.schedule;
  const scheduleDisplay = isRecurring
    ? (reminder.schedule as { Cron: string }).Cron
    : reminder.next_trigger
      ? new Date(reminder.next_trigger).toLocaleString("zh-CN")
      : "未设置";

  return (
    <aside className="inspector">
      <div className="inspector-header">
        <div className="inspector-header-title">
          {isRecurring ? (
            <RotateCcw size={18} className="icon-recurring" />
          ) : (
            <Clock size={18} className="icon-onetime" />
          )}
          <span>提醒详情</span>
        </div>
        <Button.Icon variant="ghost" onClick={onClose}>
          <X size={18} />
        </Button.Icon>
      </div>

      <div className="inspector-content">
        {reminder.paused && (
          <div className="inspector-alert">
            <AlertCircle size={14} />
            <span>此提醒已暂停</span>
          </div>
        )}

        <div className="inspector-section">
          <label>
            <FileText size={14} />
            标题
          </label>
          <Input
            value={editData.title}
            onChange={(e) => handleChange("title", e.target.value)}
            placeholder="提醒标题"
          />
        </div>

        <div className="inspector-section">
          <label>
            <Clock size={14} />
            触发时间
          </label>
          <div className="inspector-readonly">{scheduleDisplay}</div>
          <div className="inspector-hint">
            {isRecurring ? "重复提醒" : "一次性提醒"}
          </div>
        </div>

        <div className="inspector-section">
          <label>
            <Tag size={14} />
            标签
          </label>
          <Input
            value={editData.tags}
            onChange={(e) => handleChange("tags", e.target.value)}
            placeholder="用逗号分隔多个标签"
          />
          {reminder.tags.length > 0 && (
            <div className="inspector-tags">
              {reminder.tags.map((tag) => (
                <Badge key={tag} variant="secondary">
                  {tag}
                </Badge>
              ))}
            </div>
          )}
        </div>

        <div className="inspector-section">
          <label>描述</label>
          <Input.TextArea
            value={editData.description}
            onChange={(e) => handleChange("description", e.target.value)}
            placeholder="添加描述..."
            rows={4}
          />
        </div>

        <div className="inspector-section">
          <label>ID</label>
          <div className="inspector-readonly">{shortId}</div>
        </div>

        <div className="inspector-actions">
          {hasChanges && (
            <button
              className="action-item action-item-primary"
              onClick={handleSave}
            >
              <Save size={16} />
              <span>保存更改</span>
            </button>
          )}

          {!reminder.completed && (
            <>
              <button
                className="action-item"
                onClick={() => onTestTrigger(shortId)}
              >
                <Bell size={16} />
                <span>测试触发</span>
              </button>

              {reminder.paused ? (
                <button
                  className="action-item"
                  onClick={() => onResume(shortId)}
                >
                  <Play size={16} />
                  <span>恢复提醒</span>
                </button>
              ) : (
                <button
                  className="action-item"
                  onClick={() => onPause(shortId)}
                >
                  <Pause size={16} />
                  <span>暂停提醒</span>
                </button>
              )}
            </>
          )}

          <button
            className="action-item action-item-destructive"
            onClick={handleDelete}
          >
            <Trash2 size={16} />
            <span>删除提醒</span>
          </button>
        </div>
      </div>
    </aside>
  );
}
