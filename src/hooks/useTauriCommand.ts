import { invoke } from "@tauri-apps/api/core";
import { ConfigData } from "../types";

export function useTauriCommand() {
  return {
    enable: () => invoke("enable"),
    disable: () => invoke("disable"),
    estop: () => invoke("estop"),
    setMode: (mode: string) => invoke("set_mode", { mode }),
    setTeamNumber: (team: number) => invoke("set_team_number", { team }),
    setAlliance: (color: string, station: number) =>
      invoke("set_alliance", { color, station }),
    setGameData: (data: string) => invoke("set_game_data", { data }),
    setUsbConnection: (enabled: boolean) =>
      invoke("set_usb_connection", { enabled }),
    rebootRoborio: () => invoke("reboot_roborio"),
    restartRobotCode: () => invoke("restart_robot_code"),
    startPracticeMode: () => invoke("start_practice_mode"),
    stopPracticeMode: () => invoke("stop_practice_mode"),
    setPracticeTiming: (countdown: number, auto: number, delay: number, teleop: number) =>
      invoke("set_practice_timing", { countdown, auto_secs: auto, delay, teleop }),
    reorderJoysticks: (order: string[]) => invoke("reorder_joysticks", { order }),
    lockJoystick: (uuid: string, slot: number) =>
      invoke("lock_joystick", { uuid, slot }),
    unlockJoystick: (uuid: string) => invoke("unlock_joystick", { uuid }),
    rescanJoysticks: () => invoke("rescan_joysticks"),
    launchDashboard: () => invoke("launch_dashboard"),
    getConfig: () => invoke<ConfigData>("get_config"),
    saveConfig: () => invoke("save_config"),
  };
}
