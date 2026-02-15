export interface RobotState {
  connected: boolean;
  code_running: boolean;
  voltage: number;
  brownout: boolean;
  estopped: boolean;
  enabled: boolean;
  mode: string;
  joysticks: JoystickInfo[];
  any_joystick_connected: boolean;
  practice_phase: string;
  practice_elapsed_secs: number;
  practice_remaining_secs: number;
  trip_time_ms: number;
  lost_packets: number;
  team_number: number;
  alliance_color: string;
  alliance_station: number;
}

export interface JoystickInfo {
  slot: number;
  uuid: string;
  name: string;
  locked: boolean;
  connected: boolean;
  axis_count: number;
  button_count: number;
  pov_count: number;
  axes: number[];
  buttons: boolean[];
  povs: number[];
}

export interface TcpMessagePayload {
  type: "message" | "stdout" | "error" | "warning" | "version";
  text?: string;
  details?: string;
  location?: string;
  name?: string;
  version?: string;
}

export interface ConfigData {
  team_number: number;
  use_usb: boolean;
  dashboard_command: string | null;
  game_data: string;
  practice_timing: {
    countdown_secs: number;
    auto_secs: number;
    delay_secs: number;
    teleop_secs: number;
  };
  practice_audio: boolean;
}

export const INITIAL_STATE: RobotState = {
  connected: false,
  code_running: false,
  voltage: 0,
  brownout: false,
  estopped: false,
  enabled: false,
  mode: "Teleoperated",
  joysticks: [],
  any_joystick_connected: false,
  practice_phase: "Idle",
  practice_elapsed_secs: 0,
  practice_remaining_secs: 0,
  trip_time_ms: 0,
  lost_packets: 0,
  team_number: 0,
  alliance_color: "Red",
  alliance_station: 1,
};
