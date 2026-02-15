import { RobotState } from "../types";

interface Props {
  state: RobotState;
}

function getStatusString(state: RobotState): string {
  if (!state.connected) return "No Robot Communication";
  if (state.estopped) return "Emergency Stopped";
  if (state.brownout) return "Voltage Brownout";
  if (!state.code_running) return "No Robot Code";
  if (state.enabled) return `${state.mode} Enabled`;
  return `${state.mode} Disabled`;
}

function voltageColor(voltage: number, brownout: boolean): string {
  if (brownout) return "text-red-500 bg-red-900/30";
  if (voltage >= 12.5) return "text-green-400";
  if (voltage >= 8.5) return "text-yellow-400";
  return "text-red-400";
}

function StatusIndicator({ label, active }: { label: string; active: boolean }) {
  return (
    <div className="flex flex-col items-center gap-1">
      <div
        className={`w-4 h-4 rounded-full border-2 ${
          active
            ? "bg-green-500 border-green-400"
            : "bg-red-500/30 border-red-400/50"
        }`}
      />
      <span className="text-[10px] text-gray-400 uppercase tracking-wider">
        {label}
      </span>
    </div>
  );
}

export default function StatusPane({ state }: Props) {
  const statusString = getStatusString(state);
  const vColor = voltageColor(state.voltage, state.brownout);

  return (
    <div className="flex flex-col items-center gap-4 px-8 py-4">
      {/* Team Number */}
      <div className="text-3xl font-mono font-bold text-gray-200">
        {state.team_number || "----"}
      </div>

      {/* Battery Voltage */}
      <div className={`text-4xl font-mono font-bold ${vColor} px-4 py-1 rounded`}>
        {state.voltage > 0 ? state.voltage.toFixed(2) : "--.--."}
        <span className="text-lg ml-1">V</span>
      </div>

      {/* Status Indicators */}
      <div className="flex gap-6">
        <StatusIndicator label="Comms" active={state.connected} />
        <StatusIndicator label="Code" active={state.code_running} />
        <StatusIndicator label="Joysticks" active={state.any_joystick_connected} />
      </div>

      {/* Status String */}
      <div
        className={`text-sm font-semibold px-3 py-1 rounded ${
          state.estopped
            ? "text-red-400 bg-red-900/20"
            : state.enabled
            ? "text-green-400 bg-green-900/20"
            : state.brownout
            ? "text-orange-400 bg-orange-900/20"
            : "text-gray-400"
        }`}
      >
        {statusString}
      </div>
    </div>
  );
}
