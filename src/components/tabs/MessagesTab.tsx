import { useState, useEffect, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import { TcpMessagePayload } from "../../types";

interface LogEntry {
  id: number;
  timestamp: string;
  text: string;
  level: "info" | "warning" | "error";
}

let nextId = 0;
const MAX_ENTRIES = 1000;

export default function MessagesTab() {
  const [entries, setEntries] = useState<LogEntry[]>([]);
  const [autoScroll, setAutoScroll] = useState(true);
  const bottomRef = useRef<HTMLDivElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const now = () => new Date().toLocaleTimeString("en-US", { hour12: false });

    const unlistenStdout = listen<string>("stdout-message", (e) => {
      setEntries((prev) => {
        const next = [...prev, { id: nextId++, timestamp: now(), text: e.payload, level: "info" as const }];
        return next.length > MAX_ENTRIES ? next.slice(-MAX_ENTRIES) : next;
      });
    });

    const unlistenTcp = listen<TcpMessagePayload>("tcp-message", (e) => {
      const msg = e.payload;
      const level = msg.type === "error" ? "error" : msg.type === "warning" ? "warning" : "info";
      const text = msg.text || msg.details || `${msg.name} ${msg.version}` || "";
      setEntries((prev) => {
        const next = [...prev, { id: nextId++, timestamp: now(), text, level: level as "info" | "warning" | "error" }];
        return next.length > MAX_ENTRIES ? next.slice(-MAX_ENTRIES) : next;
      });
    });

    return () => {
      unlistenStdout.then((f) => f());
      unlistenTcp.then((f) => f());
    };
  }, []);

  useEffect(() => {
    if (autoScroll && bottomRef.current) {
      bottomRef.current.scrollIntoView({ behavior: "smooth" });
    }
  }, [entries, autoScroll]);

  const handleScroll = () => {
    if (!containerRef.current) return;
    const { scrollTop, scrollHeight, clientHeight } = containerRef.current;
    setAutoScroll(scrollHeight - scrollTop - clientHeight < 30);
  };

  const levelColor = (level: string) => {
    switch (level) {
      case "error": return "text-red-400";
      case "warning": return "text-yellow-400";
      default: return "text-gray-300";
    }
  };

  return (
    <div className="flex flex-col h-full">
      <div className="flex items-center justify-between mb-2">
        <div className="text-xs text-gray-500 uppercase tracking-wider">Messages</div>
        <button
          onClick={() => setEntries([])}
          className="text-[10px] px-2 py-0.5 rounded bg-[#2a2a2a] hover:bg-[#333] border border-gray-600 text-gray-400"
        >
          Clear
        </button>
      </div>
      <div
        ref={containerRef}
        onScroll={handleScroll}
        className="flex-1 overflow-y-auto font-mono text-[11px] bg-[#1a1a1a] rounded p-1.5 min-h-0"
      >
        {entries.length === 0 && (
          <div className="text-gray-600 text-center py-4">No messages</div>
        )}
        {entries.map((entry) => (
          <div key={entry.id} className={`${levelColor(entry.level)} leading-tight`}>
            <span className="text-gray-600">{entry.timestamp}</span>{" "}
            {entry.text}
          </div>
        ))}
        <div ref={bottomRef} />
      </div>
    </div>
  );
}
