use tokio::sync::{mpsc, watch};
use crate::types::*;
use crate::connection::{ConnectionManager, ConnectionState};
use crate::packet::incoming::RioPacket;
use crate::packet::tcp;

/// Internal channels needed to run the driver station
struct DsChannels {
    control_rx: mpsc::UnboundedReceiver<(ControlFlags, RequestFlags, Vec<JoystickData>, Alliance)>,
    tcp_outbound_rx: mpsc::UnboundedReceiver<Vec<u8>>,
    packet_tx: mpsc::UnboundedSender<RioPacket>,
    tcp_message_tx: mpsc::UnboundedSender<TcpMessage>,
    packet_rx: mpsc::UnboundedReceiver<RioPacket>,
    tcp_message_rx: mpsc::UnboundedReceiver<TcpMessage>,
    state_tx: watch::Sender<RobotState>,
    stdout_tx: mpsc::UnboundedSender<String>,
    messages_tx: mpsc::UnboundedSender<TcpMessage>,
}

/// The main driver station protocol handler
pub struct DriverStation {
    team: u32,
    alliance: Alliance,
    control: ControlFlags,
    request: RequestFlags,
    joysticks: Vec<JoystickData>,
    game_data: String,
    use_usb: bool,
    estopped: bool,

    // Channel to send control updates to ConnectionManager
    control_tx: mpsc::UnboundedSender<(ControlFlags, RequestFlags, Vec<JoystickData>, Alliance)>,
    // Channel to send outbound TCP frames
    tcp_outbound_tx: mpsc::UnboundedSender<Vec<u8>>,

    // Internal channels - taken by run()
    channels: Option<DsChannels>,
}

/// Receiver handle for consuming DS events — given to the application layer
pub struct DsReceiver {
    /// Watch channel for robot state updates
    pub state: watch::Receiver<RobotState>,
    /// Stdout lines from robot
    pub stdout: mpsc::UnboundedReceiver<String>,
    /// TCP messages (errors, version info, etc.)
    pub messages: mpsc::UnboundedReceiver<TcpMessage>,
}

impl DriverStation {
    /// Create a new DriverStation. Returns the DS instance and a receiver for events.
    /// Does NOT start communication — call `run()` to start.
    pub fn new(team: u32, alliance: Alliance) -> (Self, DsReceiver) {
        // Create all channels:
        // - control_tx/rx for sending control state to ConnectionManager
        let (control_tx, control_rx) = mpsc::unbounded_channel();

        // - tcp_outbound_tx/rx for sending TCP frames
        let (tcp_outbound_tx, tcp_outbound_rx) = mpsc::unbounded_channel();

        // - packet_tx/rx for receiving parsed UDP packets from ConnectionManager
        let (packet_tx, packet_rx) = mpsc::unbounded_channel();

        // - tcp_message_tx/rx for receiving parsed TCP messages
        let (tcp_message_tx, tcp_message_rx) = mpsc::unbounded_channel();

        // - state watch channel for RobotState
        let initial_state = RobotState {
            connected: false,
            code_running: false,
            voltage: BatteryVoltage { volts: 0.0 },
            status: StatusFlags {
                estop: false,
                code_initializing: false,
                brownout: false,
                enabled: false,
                mode: Mode::Teleop,
            },
            telemetry: TelemetryData::default(),
            sequence: 0,
            trip_time_ms: 0.0,
            lost_packets: 0,
        };
        let (state_tx, state_rx) = watch::channel(initial_state);

        // - stdout mpsc for stdout lines
        let (stdout_tx, stdout_rx) = mpsc::unbounded_channel();

        // - messages mpsc for other TCP messages
        let (messages_tx, messages_rx) = mpsc::unbounded_channel();

        let channels = DsChannels {
            control_rx,
            tcp_outbound_rx,
            packet_tx,
            tcp_message_tx,
            packet_rx,
            tcp_message_rx,
            state_tx,
            stdout_tx,
            messages_tx,
        };

        let ds = DriverStation {
            team,
            alliance,
            control: ControlFlags::default(),
            request: RequestFlags::default(),
            joysticks: Vec::new(),
            game_data: String::new(),
            use_usb: false,
            estopped: false,
            control_tx,
            tcp_outbound_tx,
            channels: Some(channels),
        };

        let receiver = DsReceiver {
            state: state_rx,
            stdout: stdout_rx,
            messages: messages_rx,
        };

        (ds, receiver)
    }

    /// Start the protocol communication. This spawns background tasks and runs forever.
    /// Call this once, it will manage connection/reconnection internally.
    pub async fn run(&mut self) {
        let mut channels = self.channels.take().expect("run() called more than once");

        // Send initial control state
        let _ = self.control_tx.send((
            self.control,
            self.request,
            self.joysticks.clone(),
            self.alliance,
        ));

        // 1. Create ConnectionManager
        let mut conn_mgr = ConnectionManager::new(self.team);
        conn_mgr.set_usb_mode(self.use_usb);

        // 2. Spawn ConnectionManager::run()
        tokio::spawn(async move {
            conn_mgr.run(
                channels.control_rx,
                channels.packet_tx,
                channels.tcp_message_tx,
                channels.tcp_outbound_rx,
            ).await;
        });

        // 3. Spawn a task that reads from packet_rx (RioPackets from UDP):
        //    - Update RobotState from each packet
        //    - Send updated state via watch channel
        let state_tx = channels.state_tx.clone();
        tokio::spawn(async move {
            let mut current_state = RobotState {
                connected: false,
                code_running: false,
                voltage: BatteryVoltage { volts: 0.0 },
                status: StatusFlags {
                    estop: false,
                    code_initializing: false,
                    brownout: false,
                    enabled: false,
                    mode: Mode::Teleop,
                },
                telemetry: TelemetryData::default(),
                sequence: 0,
                trip_time_ms: 0.0,
                lost_packets: 0,
            };

            while let Some(packet) = channels.packet_rx.recv().await {
                update_robot_state(&mut current_state, &packet, ConnectionState::Connected);
                let _ = state_tx.send(current_state.clone());
            }
        });

        // 4. Spawn a task that reads from tcp_message_rx:
        //    - For Stdout messages: forward to stdout channel
        //    - For other messages: forward to messages channel
        let stdout_tx = channels.stdout_tx.clone();
        let messages_tx = channels.messages_tx.clone();
        tokio::spawn(async move {
            while let Some(msg) = channels.tcp_message_rx.recv().await {
                match &msg {
                    TcpMessage::Stdout(text) => {
                        let _ = stdout_tx.send(text.clone());
                    }
                    _ => {
                        let _ = messages_tx.send(msg);
                    }
                }
            }
        });

        // Main loop - keep running forever
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }
    }

    pub fn enable(&mut self) {
        if !self.estopped {
            self.control.enabled = true;
            self.send_control();
        }
    }

    pub fn disable(&mut self) {
        self.control.enabled = false;
        self.send_control();
    }

    pub fn estop(&mut self) {
        self.estopped = true;
        self.control.estop = true;
        self.control.enabled = false;
        self.send_control();
    }

    pub fn set_mode(&mut self, mode: Mode) {
        self.control.mode = mode;
        self.send_control();
    }

    pub fn set_team(&mut self, team: u32) {
        self.team = team;
        // Note: ConnectionManager will need to be notified (can send via a separate channel
        // or handle in run loop)
    }

    pub fn set_alliance(&mut self, alliance: Alliance) {
        self.alliance = alliance;
        self.send_control();
    }

    pub fn set_joysticks(&mut self, joysticks: Vec<JoystickData>) {
        self.joysticks = joysticks;
        self.send_control();
    }

    pub fn set_game_data(&mut self, data: String) {
        self.game_data = data.clone();
        // Send game data frame via TCP
        let frame = tcp::build_game_data_frame(&data);
        let _ = self.tcp_outbound_tx.send(frame);
    }

    pub fn set_usb_mode(&mut self, usb: bool) {
        self.use_usb = usb;
    }

    pub fn reboot_roborio(&mut self) {
        self.request.reboot_roborio = true;
        self.send_control();
        // Clear the flag after one send (it's a one-shot request)
        self.request.reboot_roborio = false;
    }

    pub fn restart_code(&mut self) {
        self.request.restart_code = true;
        self.send_control();
        self.request.restart_code = false;
    }

    pub fn is_estopped(&self) -> bool {
        self.estopped
    }

    pub fn clear_estop(&mut self) {
        self.estopped = false;
        self.control.estop = false;
        self.send_control();
    }

    /// Send current control state to ConnectionManager
    fn send_control(&self) {
        let _ = self.control_tx.send((
            self.control,
            self.request,
            self.joysticks.clone(),
            self.alliance,
        ));
    }
}

/// Update robot state from a received RioPacket
fn update_robot_state(state: &mut RobotState, packet: &RioPacket, conn_state: ConnectionState) {
    state.connected = conn_state != ConnectionState::Disconnected;
    state.code_running = !packet.status.code_initializing;
    state.voltage = packet.voltage;
    state.status = packet.status;
    state.sequence = packet.sequence;

    // Process tags for telemetry
    for tag in &packet.tags {
        match tag {
            crate::packet::incoming::RioTag::CanMetrics(can) => state.telemetry.can = *can,
            crate::packet::incoming::RioTag::PdpData(currents) => state.telemetry.pdp_currents = currents.clone(),
            crate::packet::incoming::RioTag::CpuUsage(cpu) => state.telemetry.cpu_usage = cpu.clone(),
            crate::packet::incoming::RioTag::RamUsage(ram) => state.telemetry.ram_usage = *ram,
            crate::packet::incoming::RioTag::DiskUsage(disk) => state.telemetry.disk_free = *disk,
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enable_disable() {
        let (mut ds, _rx) = DriverStation::new(1234, Alliance::new(AllianceColor::Red, 1));
        ds.enable();
        assert!(ds.control.enabled);
        ds.disable();
        assert!(!ds.control.enabled);
    }

    #[test]
    fn test_estop_persists() {
        let (mut ds, _rx) = DriverStation::new(1234, Alliance::new(AllianceColor::Red, 1));
        ds.enable();
        ds.estop();
        assert!(ds.is_estopped());
        assert!(!ds.control.enabled);
        // Trying to enable after estop should not work
        ds.enable();
        assert!(!ds.control.enabled);
    }

    #[test]
    fn test_clear_estop() {
        let (mut ds, _rx) = DriverStation::new(1234, Alliance::new(AllianceColor::Red, 1));
        ds.estop();
        assert!(ds.is_estopped());
        ds.clear_estop();
        assert!(!ds.is_estopped());
        ds.enable();
        assert!(ds.control.enabled);
    }

    #[test]
    fn test_mode_switching() {
        let (mut ds, _rx) = DriverStation::new(1234, Alliance::new(AllianceColor::Red, 1));
        ds.set_mode(Mode::Autonomous);
        assert_eq!(ds.control.mode, Mode::Autonomous);
        ds.set_mode(Mode::Test);
        assert_eq!(ds.control.mode, Mode::Test);
        ds.set_mode(Mode::Teleop);
        assert_eq!(ds.control.mode, Mode::Teleop);
    }

    #[test]
    fn test_joystick_data() {
        let (mut ds, _rx) = DriverStation::new(1234, Alliance::new(AllianceColor::Red, 1));
        let js = vec![JoystickData {
            axes: vec![0, 127, -128],
            buttons: vec![true, false, true],
            povs: vec![90],
        }];
        ds.set_joysticks(js.clone());
        assert_eq!(ds.joysticks.len(), 1);
        assert_eq!(ds.joysticks[0].axes.len(), 3);
    }

    #[test]
    fn test_game_data() {
        let (mut ds, _rx) = DriverStation::new(1234, Alliance::new(AllianceColor::Red, 1));
        ds.set_game_data("LRL".to_string());
        assert_eq!(ds.game_data, "LRL");
    }
}
