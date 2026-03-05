import { useState, useEffect, useRef } from "react";
import { Modal, Input } from "@pikoloo/darwin-ui";
import { Plus, List, Sun, Pause } from "lucide-react";

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

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
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
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [isOpen, filteredCommands, selectedIndex, onClose]);

  const handleQueryChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    setQuery(e.target.value);
    setSelectedIndex(0);
    onSearch(e.target.value);
  };

  return (
    <Modal
      isOpen={isOpen}
      onClose={onClose}
      size="md"
      glass
      title="命令面板"
    >
      <div className="quick-command-content">
        <Input
          ref={inputRef}
          value={query}
          onChange={handleQueryChange}
          placeholder="搜索提醒或输入命令..."
          size="lg"
        />

        <div className="quick-command-list">
          {filteredCommands.map((command, index) => (
            <button
              key={command.id}
              className={`quick-command-item ${
                index === selectedIndex ? "selected" : ""
              }`}
              onClick={command.action}
              onMouseEnter={() => setSelectedIndex(index)}
            >
              <span className="quick-command-icon">{command.icon}</span>
              <span className="quick-command-title">{command.title}</span>
              {command.shortcut && (
                <kbd className="quick-command-shortcut">{command.shortcut}</kbd>
              )}
            </button>
          ))}
        </div>

        <div className="quick-command-footer">
          <span>↑↓ 导航</span>
          <span>↵ 选择</span>
          <span>ESC 关闭</span>
        </div>
      </div>
    </Modal>
  );
}