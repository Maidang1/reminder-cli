import { useState, useEffect, useCallback, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  isPermissionGranted,
  requestPermission,
} from "@tauri-apps/plugin-notification";
import { Sidebar } from "./components/Sidebar";
import { ReminderList } from "./components/ReminderList";
import { Inspector } from "./components/Inspector";
import { SearchBar } from "./components/SearchBar";
import QuickCommand from "./components/QuickCommand";
import "./App.css";

export interface Reminder {
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

export type ViewMode =
  | "all"
  | "today"
  | "week"
  | "upcoming"
  | "paused"
  | "completed";

function App() {
  const [reminders, setReminders] = useState<Reminder[]>([]);
  const [tags, setTags] = useState<string[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [viewMode, setViewMode] = useState<ViewMode>("all");
  const [selectedTag, setSelectedTag] = useState<string | null>(null);
  const [searchQuery, setSearchQuery] = useState("");
  const [selectedReminder, setSelectedReminder] = useState<Reminder | null>(
    null,
  );
  const [showCommandPalette, setShowCommandPalette] = useState(false);
  const [isEditing, setIsEditing] = useState(false);

  // Load data
  const loadData = useCallback(async () => {
    try {
      const [remindersData, tagsData] = await Promise.all([
        invoke<Reminder[]>("get_reminders"),
        invoke<string[]>("get_tags"),
      ]);
      setReminders(remindersData);
      setTags(tagsData);
      setError(null);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    loadData();
    const interval = setInterval(loadData, 30000); // 30s refresh
    return () => clearInterval(interval);
  }, [loadData]);

  // Keyboard shortcuts
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // Cmd/Ctrl + K: Search
      if ((e.metaKey || e.ctrlKey) && e.key === "k") {
        e.preventDefault();
        setShowCommandPalette(true);
      }
      // Cmd/Ctrl + N: New reminder
      if ((e.metaKey || e.ctrlKey) && e.key === "n") {
        e.preventDefault();
        handleNewReminder();
      }
      // Cmd/Ctrl + 1/2/3: Switch views
      if ((e.metaKey || e.ctrlKey) && e.key >= "1" && e.key <= "4") {
        e.preventDefault();
        const views: ViewMode[] = ["all", "today", "week", "upcoming"];
        setViewMode(views[parseInt(e.key) - 1]);
      }
      // Escape: Clear selection
      if (e.key === "Escape") {
        if (isEditing) {
          setIsEditing(false);
        } else {
          setSelectedReminder(null);
        }
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [isEditing]);

  // Filter reminders
  const filteredReminders = useMemo(() => {
    let result = reminders;

    // View filter
    const now = new Date();
    const today = new Date(now.getFullYear(), now.getMonth(), now.getDate());
    const weekLater = new Date(today);
    weekLater.setDate(weekLater.getDate() + 7);

    switch (viewMode) {
      case "today":
        result = result.filter((r) => {
          if (!r.next_trigger) return false;
          const trigger = new Date(r.next_trigger);
          return (
            trigger >= today && trigger < new Date(today.getTime() + 86400000)
          );
        });
        break;
      case "week":
        result = result.filter((r) => {
          if (!r.next_trigger) return false;
          const trigger = new Date(r.next_trigger);
          return trigger >= today && trigger < weekLater;
        });
        break;
      case "upcoming":
        result = result.filter(
          (r) => !r.completed && !r.paused && r.next_trigger,
        );
        break;
      case "paused":
        result = result.filter((r) => r.paused);
        break;
      case "completed":
        result = result.filter((r) => r.completed);
        break;
    }

    // Tag filter
    if (selectedTag) {
      result = result.filter((r) => r.tags.includes(selectedTag));
    }

    // Search filter
    if (searchQuery) {
      const query = searchQuery.toLowerCase();
      result = result.filter(
        (r) =>
          r.title.toLowerCase().includes(query) ||
          r.description?.toLowerCase().includes(query) ||
          r.tags.some((t) => t.toLowerCase().includes(query)),
      );
    }

    return result;
  }, [reminders, viewMode, selectedTag, searchQuery]);

  // Group reminders by date
  const groupedReminders = useMemo(() => {
    const groups: { [key: string]: Reminder[] } = {
      Overdue: [],
      Today: [],
      Tomorrow: [],
      "This Week": [],
      Later: [],
      "No Date": [],
      Recurring: [],
    };

    const now = new Date();
    const today = new Date(now.getFullYear(), now.getMonth(), now.getDate());
    const tomorrow = new Date(today);
    tomorrow.setDate(tomorrow.getDate() + 1);
    const weekLater = new Date(today);
    weekLater.setDate(weekLater.getDate() + 7);

    filteredReminders.forEach((r) => {
      if ("Cron" in r.schedule) {
        groups["Recurring"].push(r);
        return;
      }

      if (!r.next_trigger) {
        groups["No Date"].push(r);
        return;
      }

      const trigger = new Date(r.next_trigger);

      if (trigger < today && !r.completed) {
        groups["Overdue"].push(r);
      } else if (trigger >= today && trigger < tomorrow) {
        groups["Today"].push(r);
      } else if (
        trigger >= tomorrow &&
        trigger < new Date(tomorrow.getTime() + 86400000)
      ) {
        groups["Tomorrow"].push(r);
      } else if (trigger >= today && trigger < weekLater) {
        groups["This Week"].push(r);
      } else {
        groups["Later"].push(r);
      }
    });

    return groups;
  }, [filteredReminders]);

  // Actions
  const handleNewReminder = () => {
    setSelectedReminder(null);
    setIsEditing(true);
  };

  const handleUpdateReminder = async (
    _id: string,
    _updates: Partial<Reminder>,
  ) => {
    try {
      // Note: Backend doesn't support update yet, would need to add
      // For now, we'll just refresh
      await loadData();
    } catch (e) {
      setError(String(e));
    }
  };

  const handleDeleteReminder = async (id: string) => {
    try {
      await invoke("delete_reminder", { id });
      await loadData();
      if (selectedReminder?.id === id) {
        setSelectedReminder(null);
      }
    } catch (e) {
      setError(String(e));
    }
  };

  const handlePauseReminder = async (id: string) => {
    try {
      await invoke("pause_reminder", { id });
      await loadData();
    } catch (e) {
      setError(String(e));
    }
  };

  const handleResumeReminder = async (id: string) => {
    try {
      await invoke("resume_reminder", { id });
      await loadData();
    } catch (e) {
      setError(String(e));
    }
  };

  const handleTestTrigger = async (id: string) => {
    try {
      // Request notification permission if not granted
      let permissionGranted = await isPermissionGranted();
      if (!permissionGranted) {
        const permission = await requestPermission();
        permissionGranted = permission === "granted";
      }
      
      if (!permissionGranted) {
        setError("Notification permission not granted");
        return;
      }
      
      await invoke("test_trigger", { id });
    } catch (e) {
      setError(String(e));
    }
  };

  return (
    <div className="app">
      <Sidebar
        viewMode={viewMode}
        onViewModeChange={(view) => setViewMode(view as ViewMode)}
        tags={tags}
        selectedTag={selectedTag}
        onSelectTag={setSelectedTag}
        onNewReminder={handleNewReminder}
        reminderCounts={{
          all: reminders.length,
          today: reminders.filter((r) => {
            if (!r.next_trigger) return false;
            const today = new Date();
            today.setHours(0, 0, 0, 0);
            const trigger = new Date(r.next_trigger);
            return (
              trigger >= today && trigger < new Date(today.getTime() + 86400000)
            );
          }).length,
          week: reminders.filter((r) => {
            if (!r.next_trigger) return false;
            const today = new Date();
            today.setHours(0, 0, 0, 0);
            const weekLater = new Date(today);
            weekLater.setDate(weekLater.getDate() + 7);
            const trigger = new Date(r.next_trigger);
            return trigger >= today && trigger < weekLater;
          }).length,
          upcoming: reminders.filter((r) => !r.completed && !r.paused).length,
          paused: reminders.filter((r) => r.paused).length,
          completed: reminders.filter((r) => r.completed).length,
        }}
      />

      <main className="main-content">
        <SearchBar
          value={searchQuery}
          onChange={setSearchQuery}
        />

        {error && (
          <div className="error-banner" onClick={() => setError(null)}>
            {error}
            <button className="error-close">×</button>
          </div>
        )}

        <ReminderList
          groups={groupedReminders}
          selectedId={selectedReminder?.id ?? null}
          onSelect={setSelectedReminder}
          onPause={handlePauseReminder}
          onResume={handleResumeReminder}
          onDelete={handleDeleteReminder}
          onTest={handleTestTrigger}
          loading={loading}
        />
      </main>

      <Inspector
        reminder={selectedReminder}
        onClose={() => {
          setSelectedReminder(null);
          setIsEditing(false);
        }}
        onUpdate={handleUpdateReminder}
        onDelete={handleDeleteReminder}
        onPause={handlePauseReminder}
        onResume={handleResumeReminder}
        onTestTrigger={handleTestTrigger}
      />

      {showCommandPalette && (
        <QuickCommand
          isOpen={showCommandPalette}
          onClose={() => setShowCommandPalette(false)}
          onSearch={setSearchQuery}
          onCreateNew={handleNewReminder}
          remindersCount={reminders.length}
        />
      )}

      <div className="keyboard-hint">
        <kbd>⌘</kbd>
        <kbd>K</kbd> Command
        <kbd>⌘</kbd>
        <kbd>N</kbd> New
        <kbd>⌘</kbd>
        <kbd>1~4</kbd> Views
      </div>
    </div>
  );
}

export default App;
