import { RobotState } from "../../types";
import { useTauriCommand } from "../../hooks/useTauriCommand";

interface Props {
  state: RobotState;
}

function Indicator({ label, active }: { label: string; active: boolean }) {
  return (
    <div className="flex items-center gap-2">
      <div className={`w-2.5 h-2.5 rounded-full ${active ? "bg-green-500" : "bg-gray-600"}`} />
      <span className="text-xs text-gray-300">{label}</span>
    </div>
  );
}

export default function DiagnosticsTab({ state }: Props) {
  const cmd = useTauriCommand();

  return (
    <div className="flex flex-col gap-3">
      <div className="text-xs text-gray-500 uppercase tracking-wider">Version</div>
      <div className="text-xs text-gray-400">Open Station v0.1.0</div>

      <div className="text-xs text-gray-500 uppercase tracking-wider mt-2">Connection</div>
      <div className="flex flex-col gap-1.5">
        <Indicator label="Robot" active={state.connected} />
        <Indicator label="Robot Code" active={state.code_running} />
      </div>

      <div className="text-xs text-gray-500 uppercase tracking-wider mt-2">Memory</div>
      <div className="text-xs text-gray-400">
        RAM: {state.connected ? `${(state.trip_time_ms).toFixed(0)} ms trip` : "N/A"}
      </div>

      <div className="flex flex-col gap-2 mt-3">
        <button
          onClick={() => { if (confirm("Reboot roboRIO?")) cmd.rebootRoborio(); }}
          disabled={!state.connected}
          className="px-2 py-1.5 rounded text-xs bg-[#2a2a2a] hover:bg-[#333] disabled:opacity-50 disabled:cursor-not-allowed border border-gray-600"
        >
          Reboot roboRIO
        </button>
        <button
          onClick={() => cmd.restartRobotCode()}
          disabled={!state.connected}
          className="px-2 py-1.5 rounded text-xs bg-[#2a2a2a] hover:bg-[#333] disabled:opacity-50 disabled:cursor-not-allowed border border-gray-600"
        >
          Restart Robot Code
        </button>
      </div>
    </div>
  );
}
