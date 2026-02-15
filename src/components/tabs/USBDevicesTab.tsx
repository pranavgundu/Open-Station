import { useState } from "react";
import { RobotState, JoystickInfo } from "../../types";
import { useTauriCommand } from "../../hooks/useTauriCommand";

interface Props {
  state: RobotState;
}

const AXIS_LABELS = ["X", "Y", "Z", "Rz", "Rx", "Ry"];
const BUTTON_LABELS = ["1", "2", "3", "4", "LB", "RB", "Bk", "St", "LS", "RS"];

function AxisBar({ label, value }: { label: string; value: number }) {
  // value is -128..127, normalize to -1..1
  const norm = value / 127;
  const pct = Math.abs(norm) * 50;
  const isNeg = norm < 0;

  return (
    <div className="flex items-center gap-1.5 h-4">
      <span className="text-[9px] text-gray-500 w-4 text-right font-mono">{label}</span>
      <div className="flex-1 h-2.5 bg-[#1a1a1a] rounded-sm relative overflow-hidden">
        {/* Center line */}
        <div className="absolute left-1/2 top-0 bottom-0 w-px bg-gray-700" />
        {/* Value bar */}
        <div
          className="absolute top-0 bottom-0 bg-green-500 rounded-sm transition-all duration-75"
          style={{
            left: isNeg ? `${50 - pct}%` : "50%",
            width: `${pct}%`,
          }}
        />
      </div>
      <span className="text-[9px] text-gray-600 w-7 text-right font-mono">{value}</span>
    </div>
  );
}

function ButtonGrid({ buttons }: { buttons: boolean[] }) {
  return (
    <div className="flex flex-wrap gap-1">
      {buttons.map((pressed, i) => (
        <div
          key={i}
          className={`w-6 h-5 rounded text-[9px] font-mono flex items-center justify-center transition-colors duration-75 ${
            pressed
              ? "bg-green-500 text-black font-bold"
              : "bg-[#1a1a1a] text-gray-600"
          }`}
        >
          {BUTTON_LABELS[i] ?? i}
        </div>
      ))}
    </div>
  );
}

function PovIndicator({ value }: { value: number }) {
  // value: -1 = unpressed, 0=N, 45=NE, 90=E, etc.
  const active = value >= 0;
  const angleDeg = active ? value : 0;

  return (
    <div className="flex items-center gap-1.5">
      <span className="text-[9px] text-gray-500 font-mono">POV</span>
      <div className="w-7 h-7 rounded-full bg-[#1a1a1a] relative border border-gray-700">
        {/* Cardinal markers */}
        {[0, 90, 180, 270].map((deg) => (
          <div
            key={deg}
            className="absolute w-0.5 h-1 bg-gray-700"
            style={{
              top: deg === 180 ? "auto" : deg === 0 ? "1px" : "50%",
              bottom: deg === 180 ? "1px" : "auto",
              left: deg === 90 ? "auto" : deg === 270 ? "1px" : "50%",
              right: deg === 90 ? "1px" : "auto",
              transform:
                deg === 0 || deg === 180
                  ? "translateX(-50%)"
                  : "translateY(-50%)",
            }}
          />
        ))}
        {/* Direction indicator */}
        {active && (
          <div
            className="absolute w-1.5 h-1.5 bg-green-500 rounded-full"
            style={{
              top: "50%",
              left: "50%",
              transform: `translate(-50%, -50%) translate(${
                Math.sin((angleDeg * Math.PI) / 180) * 8
              }px, ${-Math.cos((angleDeg * Math.PI) / 180) * 8}px)`,
            }}
          />
        )}
      </div>
      <span className="text-[9px] text-gray-600 font-mono">
        {active ? `${value}°` : "—"}
      </span>
    </div>
  );
}

function JoystickDetail({ js }: { js: JoystickInfo }) {
  const axes = js.axes.length > 0 ? js.axes : Array(js.axis_count).fill(0);
  const buttons =
    js.buttons.length > 0 ? js.buttons : Array(js.button_count).fill(false);
  const povs = js.povs.length > 0 ? js.povs : [-1];

  return (
    <div className="flex flex-col gap-2 p-2 bg-[#222] rounded border border-gray-700">
      <div className="text-[10px] text-gray-400 truncate">
        {js.name} {js.locked && <span className="text-gray-600">(locked)</span>}
      </div>

      {/* Axes */}
      <div className="flex flex-col gap-0.5">
        <div className="text-[9px] text-gray-600 uppercase">Axes</div>
        {axes.map((v, i) => (
          <AxisBar key={i} label={AXIS_LABELS[i] ?? `${i}`} value={v} />
        ))}
      </div>

      {/* Buttons */}
      <div className="flex flex-col gap-1">
        <div className="text-[9px] text-gray-600 uppercase">Buttons</div>
        <ButtonGrid buttons={buttons} />
      </div>

      {/* POV */}
      <div className="flex flex-col gap-1">
        <div className="text-[9px] text-gray-600 uppercase">POV</div>
        <div className="flex gap-2">
          {povs.map((v, i) => (
            <PovIndicator key={i} value={v} />
          ))}
        </div>
      </div>
    </div>
  );
}

export default function USBDevicesTab({ state }: Props) {
  const cmd = useTauriCommand();
  const [selectedSlot, setSelectedSlot] = useState<number>(0);

  const selectedJs = state.joysticks.find((j) => j.slot === selectedSlot);

  return (
    <div className="flex flex-col gap-3">
      <div className="flex items-center justify-between">
        <div className="text-xs text-gray-500 uppercase tracking-wider">
          Joysticks
        </div>
        <button
          onClick={() => cmd.rescanJoysticks()}
          className="text-[10px] px-2 py-0.5 rounded bg-[#2a2a2a] hover:bg-[#333] border border-gray-600 text-gray-400"
        >
          Rescan (F1)
        </button>
      </div>

      {/* Slot list */}
      <div className="flex flex-col gap-1">
        {[0, 1, 2, 3, 4, 5].map((slot) => {
          const js = state.joysticks.find((j) => j.slot === slot);
          const isSelected = slot === selectedSlot;
          return (
            <button
              key={slot}
              onClick={() => setSelectedSlot(slot)}
              onDoubleClick={() => {
                if (js) {
                  js.locked
                    ? cmd.unlockJoystick(js.uuid)
                    : cmd.lockJoystick(js.uuid, slot);
                }
              }}
              className={`flex items-center gap-2 px-2 py-1.5 rounded text-xs text-left transition-colors ${
                isSelected
                  ? "bg-blue-600/20 border border-blue-500/50"
                  : js?.connected
                  ? "bg-[#2a2a2a] text-gray-200 border border-transparent"
                  : js
                  ? "bg-[#222] text-gray-600 border border-transparent"
                  : "bg-[#1a1a1a] text-gray-700 border border-transparent"
              }`}
            >
              <span className="font-mono text-gray-500 w-4">{slot}</span>
              <span
                className={`flex-1 truncate ${js?.locked ? "underline" : ""}`}
              >
                {js ? js.name : "Empty"}
              </span>
              {js?.locked && (
                <span className="text-[10px] text-gray-500">locked</span>
              )}
            </button>
          );
        })}
      </div>

      {/* Live joystick visualization */}
      {selectedJs?.connected ? (
        <JoystickDetail js={selectedJs} />
      ) : (
        <div className="text-[10px] text-gray-600 text-center py-4">
          {selectedJs
            ? "Controller disconnected"
            : "No controller in this slot"}
        </div>
      )}

      {state.joysticks.length > 0 && (
        <div className="text-[10px] text-gray-600 mt-1">
          Click to select. Double-click to lock/unlock. Drag to reorder.
        </div>
      )}
    </div>
  );
}
