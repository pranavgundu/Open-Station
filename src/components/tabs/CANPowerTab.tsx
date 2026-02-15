import { RobotState } from "../../types";

interface Props {
  state: RobotState;
}

function FaultCounter({ label, value }: { label: string; value: number }) {
  return (
    <div className="flex justify-between items-center">
      <span className="text-xs text-gray-400">{label}</span>
      <span className={`text-xs font-mono ${value > 0 ? "text-red-400" : "text-gray-600"}`}>
        {value}
      </span>
    </div>
  );
}

export default function CANPowerTab({ state }: Props) {
  const can = state.connected ? {
    utilization: 0,
    bus_off_count: 0,
    tx_full_count: 0,
    rx_error_count: 0,
    tx_error_count: 0,
  } : null;

  return (
    <div className="flex flex-col gap-3">
      <div className="text-xs text-gray-500 uppercase tracking-wider">Power Faults</div>
      <div className="flex flex-col gap-1">
        <FaultCounter label="Comms Faults" value={0} />
        <FaultCounter label="12V Faults" value={0} />
        <FaultCounter label="6V Faults" value={0} />
        <FaultCounter label="5V Faults" value={0} />
        <FaultCounter label="3.3V Faults" value={0} />
      </div>

      <div className="text-xs text-gray-500 uppercase tracking-wider mt-2">CAN Bus</div>
      {can ? (
        <div className="flex flex-col gap-1">
          <div className="flex justify-between items-center">
            <span className="text-xs text-gray-400">Utilization</span>
            <span className="text-xs font-mono text-gray-300">{can.utilization}%</span>
          </div>
          <div className="w-full bg-gray-700 rounded-full h-1.5">
            <div
              className="bg-green-500 h-1.5 rounded-full"
              style={{ width: `${Math.min(can.utilization, 100)}%` }}
            />
          </div>
          <FaultCounter label="Bus Off" value={can.bus_off_count} />
          <FaultCounter label="TX Full" value={can.tx_full_count} />
          <FaultCounter label="RX Errors" value={can.rx_error_count} />
          <FaultCounter label="TX Errors" value={can.tx_error_count} />
        </div>
      ) : (
        <div className="text-xs text-gray-600">No robot connection</div>
      )}
    </div>
  );
}
