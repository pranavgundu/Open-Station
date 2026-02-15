import { useState, useEffect, useRef } from "react";
import { RobotState } from "../../types";
import { useTauriCommand } from "../../hooks/useTauriCommand";

interface Props {
  state: RobotState;
}

export default function OperationTab({ state }: Props) {
  const cmd = useTauriCommand();
  const [elapsedMs, setElapsedMs] = useState(0);
  const enabledRef = useRef(state.enabled);
  const timerRef = useRef<ReturnType<typeof setInterval>>(undefined);

  useEffect(() => {
    if (state.enabled && !enabledRef.current) {
      setElapsedMs(0);
      timerRef.current = setInterval(() => setElapsedMs(p => p + 100), 100);
    } else if (!state.enabled && enabledRef.current) {
      if (timerRef.current) clearInterval(timerRef.current);
    }
    enabledRef.current = state.enabled;
    return () => { if (timerRef.current) clearInterval(timerRef.current); };
  }, [state.enabled]);

  const formatTime = (ms: number) => {
    const s = Math.floor(ms / 1000);
    const m = Math.floor(s / 60);
    return `${m}:${(s % 60).toString().padStart(2, "0")}`;
  };

  const modes = ["Teleoperated", "Autonomous", "Test"];
  const isPractice = state.practice_phase !== "Idle" && state.practice_phase !== "Done";

  return (
    <div className="flex flex-col gap-3">
      <div className="text-xs text-gray-500 uppercase tracking-wider">Mode</div>
      <div className="flex flex-col gap-1">
        {modes.map(m => (
          <button
            key={m}
            onClick={() => cmd.setMode(m.toLowerCase())}
            className={`text-left px-2 py-1 rounded text-xs ${
              state.mode === m ? "bg-blue-600 text-white" : "bg-[#2a2a2a] text-gray-300 hover:bg-[#333]"
            }`}
          >
            {m}
          </button>
        ))}
        <button
          onClick={() => isPractice ? cmd.stopPracticeMode() : cmd.startPracticeMode()}
          className={`text-left px-2 py-1 rounded text-xs ${
            isPractice ? "bg-orange-600 text-white" : "bg-[#2a2a2a] text-gray-300 hover:bg-[#333]"
          }`}
        >
          {isPractice ? `Practice (${state.practice_phase})` : "Practice"}
        </button>
      </div>

      {/* Enable/Disable */}
      <div className="flex gap-2 mt-2">
        <button
          onClick={() => cmd.enable()}
          disabled={!state.connected || !state.code_running || state.estopped}
          className="flex-1 py-2 rounded font-bold text-xs bg-green-600 hover:bg-green-500 disabled:bg-green-900 disabled:text-green-700 disabled:cursor-not-allowed"
        >
          Enable
        </button>
        <button
          onClick={() => cmd.disable()}
          className="flex-1 py-2 rounded font-bold text-xs bg-red-600 hover:bg-red-500"
        >
          Disable
        </button>
      </div>

      {/* Elapsed time */}
      <div className="text-center text-xs text-gray-400">
        Elapsed: <span className="font-mono">{formatTime(elapsedMs)}</span>
      </div>

      {/* Alliance */}
      <div className="text-xs text-gray-500 uppercase tracking-wider mt-2">Alliance Station</div>
      <select
        value={`${state.alliance_color}${state.alliance_station}`}
        onChange={(e) => {
          const v = e.target.value;
          const color = v.startsWith("Red") ? "Red" : "Blue";
          const station = parseInt(v.slice(-1));
          cmd.setAlliance(color, station);
        }}
        className="bg-[#2a2a2a] border border-gray-600 rounded px-2 py-1 text-xs"
      >
        {["Red", "Blue"].flatMap(c => [1, 2, 3].map(s => (
          <option key={`${c}${s}`} value={`${c}${s}`}>{c} {s}</option>
        )))}
      </select>
    </div>
  );
}
