use crate::connection::{ConnectionManager, ConnectionState};
use crate::packet::incoming::RioPacket;
use crate::packet::tcp;
use crate::types::*;
use tokio::sync::{mpsc, watch};

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

pub struct DriverStation {
    team: u32,
    alliance: Alliance,
    control: ControlFlags,
    request: RequestFlags,
    joysticks: Vec<JoystickData>,
    game_data: String,
    use_usb: bool,
    estopped: bool,

    control_tx: mpsc::UnboundedSender<(ControlFlags, RequestFlags, Vec<JoystickData>, Alliance)>,
    tcp_outbound_tx: mpsc::UnboundedSender<Vec<u8>>,

    channels: Option<DsChannels>,
}

pub struct DsReceiver {
    pub state: watch::Receiver<RobotState>,
    pub stdout: mpsc::UnboundedReceiver<String>,
    pub messages: mpsc::UnboundedReceiver<TcpMessage>,
}

impl DriverStation {
    pub fn new(team: u32, alliance: Alliance) -> (Self, DsReceiver) {
        let (control_tx, control_rx) = mpsc::unbounded_channel();

        let (tcp_outbound_tx, tcp_outbound_rx) = mpsc::unbounded_channel();

        let (packet_tx, packet_rx) = mpsc::unbounded_channel();

        let (tcp_message_tx, tcp_message_rx) = mpsc::unbounded_channel();

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

        let (stdout_tx, stdout_rx) = mpsc::unbounded_channel();

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

    pub async fn run(&mut self) {
        let mut channels = self.channels.take().expect("run() called more than once");

        let _ = self.control_tx.send((
            self.control,
            self.request,
            self.joysticks.clone(),
            self.alliance,
        ));

        let mut conn_mgr = ConnectionManager::new(self.team);
        conn_mgr.set_usb_mode(self.use_usb);

        tokio::spawn(async move {
            conn_mgr
                .run(
                    channels.control_rx,
                    channels.packet_tx,
                    channels.tcp_message_tx,
                    channels.tcp_outbound_rx,
                )
                .await;
        });

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

    fn send_control(&self) {
        let _ = self.control_tx.send((
            self.control,
            self.request,
            self.joysticks.clone(),
            self.alliance,
        ));
    }
}

fn update_robot_state(state: &mut RobotState, packet: &RioPacket, conn_state: ConnectionState) {
    state.connected = conn_state != ConnectionState::Disconnected;
    state.code_running = !packet.status.code_initializing;
    state.voltage = packet.voltage;
    state.status = packet.status;
    state.sequence = packet.sequence;

    for tag in &packet.tags {
        match tag {
            crate::packet::incoming::RioTag::CanMetrics(can) => state.telemetry.can = *can,
            crate::packet::incoming::RioTag::PdpData(currents) => {
                state.telemetry.pdp_currents = currents.clone()
            }
            crate::packet::incoming::RioTag::CpuUsage(cpu) => {
                state.telemetry.cpu_usage = cpu.clone()
            }
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
