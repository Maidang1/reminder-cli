import { useState, useEffect, useRef, useCallback } from "react";
import { createPortal } from "react-dom";
import { Input } from "@pikoloo/darwin-ui";
import { Plus, List, Sun, Pause, X } from "lucide-react";

interface QuickCommandProps {
  isOpen: boolean;
  onClose: () => void;
  onSearch: (query: string) => void;
  onCreateNew: () => void;
  remindersCount: number;
}

interface Command {
  id: string;
  title: string;
  shortcut?: string;
  icon: React.ReactNode;
  action: () => void;
}

export default function QuickCommand({
  isOpen,
  onClose,
  onSearch,
  onCreateNew,
  remindersCount,
}: QuickCommandProps) {
  const [query, setQuery] = useState("");
  const [selectedIndex, setSelectedIndex] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);

  const commands: Command[] = [
    {
      id: "new",
      title: "新建提醒",
      shortcut: "⌘N",
      icon: <Plus size={16} />,
      action: () => {
        onCreateNew();
        onClose();
      },
    },
    {
      id: "all",
      title: `查看全部 (${remindersCount})`,
      icon: <List size={16} />,
      action: () => {
        onSearch("");
        onClose();
      },
    },
    {
      id: "today",
      title: "查看今天",
      icon: <Sun size={16} />,
      action: () => {
        onSearch("today");
        onClose();
      },
    },
    {
      id: "paused",
      title: "查看已暂停",
      icon: <Pause size={16} />,
      action: () => {
        onSearch("paused");
        onClose();
      },
    },
  ];

  const filteredCommands = query
    ? commands.filter(
        (c) =>
          c.title.toLowerCase().includes(query.toLowerCase()) ||
          c.id.includes(query.toLowerCase()),
      )
    : commands;

  useEffect(() => {
    if (isOpen) {
      setQuery("");
      setSelectedIndex(0);
      setTimeout(() => inputRef.current?.focus(), 100);
    }
  }, [isOpen]);

  const handleKeyDown = useCallback((e: KeyboardEvent) => {
    if (!isOpen) return;

    switch (e.key) {
      case "ArrowDown":
        e.preventDefault();
        setSelectedIndex((prev) =>
          Math.min(prev + 1, filteredCommands.length - 1),
        );
        break;
      case "ArrowUp":
        e.preventDefault();
        setSelectedIndex((prev) => Math.max(prev - 1, 0));
        break;
      case "Enter":
        e.preventDefault();
        filteredCommands[selectedIndex]?.action();
        break;
      case "Escape":
        onClose();
        break;
    }
  }, [isOpen, filteredCommands, selectedIndex, onClose]);

  useEffect(() => {
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [handleKeyDown]);

  const handleQueryChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    setQuery(e.target.value);
    setSelectedIndex(0);
    onSearch(e.target.value);
  };

  const handleBackdropClick = (e: React.MouseEvent) => {
    if (e.target === e.currentTarget) {
      onClose();
    }
  };

  if (!isOpen) return null;

  return createPortal(
    <div className="command-palette-overlay" onClick={handleBackdropClick}>
      <div className="command-palette">
        <div className="command-palette-header">
          <Input
            ref={inputRef}
            value={query}
            onChange={handleQueryChange}
            placeholder="搜索提醒或输入命令..."
            size="lg"
          />
          <button className="command-palette-close" onClick={onClose}>
            <X size={16} />
          </button>
        </div>

        <div className="command-palette-list">
          {filteredCommands.map((command, index) => (
            <button
              key={command.id}
              className={`command-palette-item ${
                index === selectedIndex ? "selected" : ""
              }`}
              onClick={command.action}
              onMouseEnter={() => setSelectedIndex(index)}
            >
              <span className="command-palette-icon">{command.icon}</span>
              <span className="command-palette-title">{command.title}</span>
              {command.shortcut && (
                <kbd className="command-palette-shortcut">{command.shortcut}</kbd>
              )}
            </button>
          ))}
        </div>

        <div className="command-palette-footer">
          <span>↑↓ 导航</span>
          <span>↵ 选择</span>
          <span>ESC 关闭</span>
        </div>
      </div>
    </div>,
    document.body,
  );
}
