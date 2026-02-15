import { RobotState } from "../../types";
import { useTauriCommand } from "../../hooks/useTauriCommand";

interface Props {
  state: RobotState;
}

export default function USBDevicesTab({ state }: Props) {
  const cmd = useTauriCommand();

  return (
    <div className="flex flex-col gap-3">
      <div className="flex items-center justify-between">
        <div className="text-xs text-gray-500 uppercase tracking-wider">Joysticks</div>
        <button
          onClick={() => cmd.rescanJoysticks()}
          className="text-[10px] px-2 py-0.5 rounded bg-[#2a2a2a] hover:bg-[#333] border border-gray-600 text-gray-400"
        >
          Rescan (F1)
        </button>
      </div>

      <div className="flex flex-col gap-1">
        {[0, 1, 2, 3, 4, 5].map((slot) => {
          const js = state.joysticks.find((j) => j.slot === slot);
          return (
            <div
              key={slot}
              className={`flex items-center gap-2 px-2 py-1.5 rounded text-xs ${
                js?.connected
                  ? "bg-[#2a2a2a] text-gray-200"
                  : js
                  ? "bg-[#222] text-gray-600"
                  : "bg-[#1a1a1a] text-gray-700"
              }`}
              onDoubleClick={() => {
                if (js) {
                  js.locked ? cmd.unlockJoystick(js.uuid) : cmd.lockJoystick(js.uuid, slot);
                }
              }}
            >
              <span className="font-mono text-gray-500 w-4">{slot}</span>
              <span className={`flex-1 truncate ${js?.locked ? "underline" : ""}`}>
                {js ? js.name : "Empty"}
              </span>
              {js?.locked && <span className="text-[10px] text-gray-500">locked</span>}
            </div>
          );
        })}
      </div>

      {state.joysticks.length > 0 && (
        <div className="text-[10px] text-gray-600 mt-1">
          Double-click to lock/unlock. Drag to reorder.
        </div>
      )}
    </div>
  );
}
