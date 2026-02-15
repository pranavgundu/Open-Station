import { useState, useEffect } from "react";
import { RobotState } from "../../types";
import { useTauriCommand } from "../../hooks/useTauriCommand";

interface Props {
  state: RobotState;
}

export default function SetupTab({ state }: Props) {
  const cmd = useTauriCommand();
  const [teamInput, setTeamInput] = useState(state.team_number.toString());
  const [gameData, setGameData] = useState("");
  const [useUsb, setUseUsb] = useState(false);

  useEffect(() => {
    cmd.getConfig().then((config) => {
      setGameData(config.game_data);
      setUseUsb(config.use_usb);
    }).catch(() => {});
  }, []);

  const handleTeamSubmit = () => {
    const team = parseInt(teamInput);
    if (!isNaN(team) && team >= 0 && team <= 9999) {
      cmd.setTeamNumber(team);
      cmd.saveConfig();
    }
  };

  return (
    <div className="flex flex-col gap-3">
      <div className="text-xs text-gray-500 uppercase tracking-wider">Team Number</div>
      <input
        type="number"
        min="0"
        max="9999"
        value={teamInput}
        onChange={(e) => setTeamInput(e.target.value)}
        onBlur={handleTeamSubmit}
        onKeyDown={(e) => e.key === "Enter" && handleTeamSubmit()}
        className="bg-[#2a2a2a] border border-gray-600 rounded px-2 py-1 text-sm font-mono w-full"
      />

      <div className="text-xs text-gray-500 uppercase tracking-wider mt-2">Game Data</div>
      <input
        type="text"
        maxLength={3}
        value={gameData}
        onChange={(e) => {
          setGameData(e.target.value);
          cmd.setGameData(e.target.value);
        }}
        className="bg-[#2a2a2a] border border-gray-600 rounded px-2 py-1 text-sm font-mono w-full"
        placeholder="e.g. LRL"
      />

      <label className="flex items-center gap-2 mt-2">
        <input
          type="checkbox"
          checked={useUsb}
          onChange={(e) => {
            setUseUsb(e.target.checked);
            cmd.setUsbConnection(e.target.checked);
          }}
          className="rounded"
        />
        <span className="text-xs text-gray-300">Connect via USB</span>
      </label>

      <div className="text-xs text-gray-500 uppercase tracking-wider mt-3">Practice Timing (sec)</div>
      <PracticeTimingInputs />
    </div>
  );
}

function PracticeTimingInputs() {
  const cmd = useTauriCommand();
  const [auto, setAuto] = useState(15);
  const [delay, setDelay] = useState(1);
  const [teleop, setTeleop] = useState(135);

  const save = () => cmd.setPracticeTiming(3, auto, delay, teleop);

  return (
    <div className="grid grid-cols-2 gap-2">
      {[
        { label: "Auto", value: auto, set: setAuto },
        { label: "Delay", value: delay, set: setDelay },
        { label: "Teleop", value: teleop, set: setTeleop },
      ].map(({ label, value, set }) => (
        <div key={label}>
          <div className="text-[10px] text-gray-500">{label}</div>
          <input
            type="number"
            min="0"
            value={value}
            onChange={(e) => set(parseInt(e.target.value) || 0)}
            onBlur={save}
            className="bg-[#2a2a2a] border border-gray-600 rounded px-1.5 py-0.5 text-xs font-mono w-full"
          />
        </div>
      ))}
    </div>
  );
}
