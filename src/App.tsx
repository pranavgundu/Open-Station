import { useState } from "react";
import { useTauriEvent } from "./hooks/useTauriEvent";
import { RobotState, INITIAL_STATE } from "./types";
import StatusPane from "./components/StatusPane";
import TabBar from "./components/TabBar";
import OperationTab from "./components/tabs/OperationTab";
import DiagnosticsTab from "./components/tabs/DiagnosticsTab";
import SetupTab from "./components/tabs/SetupTab";
import USBDevicesTab from "./components/tabs/USBDevicesTab";
import CANPowerTab from "./components/tabs/CANPowerTab";
import MessagesTab from "./components/tabs/MessagesTab";
import ChartsTab from "./components/tabs/ChartsTab";
import BothTab from "./components/tabs/BothTab";

const LEFT_TABS = [
  { id: "operation", label: "Oper" },
  { id: "diagnostics", label: "Diag" },
  { id: "setup", label: "Setup" },
  { id: "usb", label: "USB" },
  { id: "canpower", label: "CAN" },
];

const RIGHT_TABS = [
  { id: "messages", label: "Msgs" },
  { id: "charts", label: "Chart" },
  { id: "both", label: "Both" },
];

export default function App() {
  const state = useTauriEvent<RobotState>("robot-state", INITIAL_STATE);
  const [leftTab, setLeftTab] = useState("operation");
  const [rightTab, setRightTab] = useState("messages");

  return (
    <div className="h-screen flex bg-[#1e1e1e] text-white text-sm select-none overflow-hidden">
      {/* Left Tab Bar */}
      <TabBar tabs={LEFT_TABS} active={leftTab} onSelect={setLeftTab} position="left" />

      {/* Left Panel */}
      <div className="w-56 border-r border-gray-700 overflow-y-auto p-3">
        <div className="text-gray-500 text-xs uppercase tracking-wider">
          {LEFT_TABS.find((t) => t.id === leftTab)?.label || ""}
        </div>
        {leftTab === "operation" && <OperationTab state={state} />}
        {leftTab === "diagnostics" && <DiagnosticsTab state={state} />}
        {leftTab === "setup" && <SetupTab state={state} />}
        {leftTab === "usb" && <USBDevicesTab state={state} />}
        {leftTab === "canpower" && <CANPowerTab state={state} />}
      </div>

      {/* Center Status Pane */}
      <div className="flex-1 flex items-center justify-center">
        <StatusPane state={state} />
      </div>

      {/* Right Panel */}
      <div className="w-72 border-l border-gray-700 overflow-y-auto p-3">
        {rightTab === "messages" && <MessagesTab />}
        {rightTab === "charts" && <ChartsTab state={state} />}
        {rightTab === "both" && <BothTab state={state} />}
      </div>

      {/* Right Tab Bar */}
      <TabBar tabs={RIGHT_TABS} active={rightTab} onSelect={setRightTab} position="right" />
    </div>
  );
}
