use gilrs::{EventType, GamepadId, Gilrs};
use open_station_protocol::types::JoystickData;
use std::collections::HashMap;

pub mod mapping;

#[derive(Debug, Clone)]
pub struct JoystickInfo {
    pub slot: u8,
    pub uuid: String,
    pub name: String,
    pub locked: bool,
    pub connected: bool,
    pub axis_count: u8,
    pub button_count: u8,
    pub pov_count: u8,
}

#[derive(Debug)]
struct JoystickSlot {
    uuid: String,
    name: String,
    gilrs_id: GamepadId,
    locked: bool,
    connected: bool,
}

#[derive(Debug)]
pub struct JoystickManager {
    gilrs: Gilrs,
    slots: Vec<Option<JoystickSlot>>,
    locks: HashMap<String, u8>,
}

impl JoystickManager {
    pub fn new(locks: HashMap<String, u8>) -> Self {
        let gilrs = Gilrs::new().expect("Failed to initialize gilrs");
        let mut manager = Self {
            gilrs,
            slots: (0..6).map(|_| None).collect(),
            locks,
        };
        manager.scan_devices();
        manager
    }

    pub fn poll(&mut self) {
        while let Some(event) = self.gilrs.next_event() {
            match event.event {
                EventType::Connected => self.on_device_connected(event.id),
                EventType::Disconnected => self.on_device_disconnected(event.id),
                _ => {}
            }
        }
    }

    pub fn get_joystick_data(&self) -> Vec<JoystickData> {
        self.slots
            .iter()
            .map(|slot| match slot {
                Some(js) if js.connected => self.read_gamepad(js.gilrs_id),
                _ => JoystickData::default(),
            })
            .collect()
    }

    pub fn get_joystick_info(&self) -> Vec<JoystickInfo> {
        self.slots
            .iter()
            .enumerate()
            .filter_map(|(i, slot)| {
                slot.as_ref().map(|js| JoystickInfo {
                    slot: i as u8,
                    uuid: js.uuid.clone(),
                    name: js.name.clone(),
                    locked: js.locked,
                    connected: js.connected,
                    axis_count: 6, // standard FRC
                    button_count: 10,
                    pov_count: 1,
                })
            })
            .collect()
    }

    pub fn reorder(&mut self, order: Vec<String>) {
        let mut new_slots: Vec<Option<JoystickSlot>> = (0..6).map(|_| None).collect();

        for (target_slot, uuid) in order.iter().enumerate() {
            if target_slot >= 6 {
                break;
            }

            // Find the device with this UUID in current slots
            for current_slot in &mut self.slots {
                if let Some(js) = current_slot.as_ref() {
                    if &js.uuid == uuid {
                        new_slots[target_slot] = current_slot.take();
                        break;
                    }
                }
            }
        }

        let mut next_empty_slot = 0;
        for current_slot in &mut self.slots {
            if let Some(js) = current_slot.take() {
                while next_empty_slot < 6 && new_slots[next_empty_slot].is_some() {
                    next_empty_slot += 1;
                }
                if next_empty_slot < 6 {
                    new_slots[next_empty_slot] = Some(js);
                    next_empty_slot += 1;
                }
            }
        }

        self.slots = new_slots;
    }

    pub fn lock(&mut self, uuid: &str, slot: u8) {
        if let Some(s) = self.slots.get_mut(slot as usize) {
            if let Some(js) = s.as_mut() {
                if js.uuid == uuid {
                    js.locked = true;
                    self.locks.insert(uuid.to_string(), slot);
                }
            }
        }
    }

    pub fn unlock(&mut self, uuid: &str) {
        self.locks.remove(uuid);
        for slot in &mut self.slots {
            if let Some(js) = slot.as_mut() {
                if js.uuid == uuid {
                    js.locked = false;
                }
            }
        }
    }

    pub fn rescan(&mut self) {
        // Clear non-locked slots
        for slot in &mut self.slots {
            if let Some(js) = slot.as_ref() {
                if !js.locked {
                    *slot = None;
                }
            }
        }
        // Re-scan for devices
        self.scan_devices();
    }

    pub fn any_connected(&self) -> bool {
        self.slots
            .iter()
            .any(|s| s.as_ref().is_some_and(|js| js.connected))
    }

    fn scan_devices(&mut self) {
        let ids: Vec<GamepadId> = self.gilrs.gamepads().map(|(id, _)| id).collect();
        for id in ids {
            self.on_device_connected(id);
        }
    }

    fn on_device_connected(&mut self, id: GamepadId) {
        let gamepad = self.gilrs.gamepad(id);

        let uuid = self.uuid_for_gamepad(id);
        let name = gamepad.name().to_string();

        for slot in &mut self.slots {
            if let Some(js) = slot.as_mut() {
                if js.uuid == uuid {
                    js.connected = true;
                    js.gilrs_id = id;
                    return;
                }
            }
        }

        if let Some(&preferred_slot) = self.locks.get(&uuid) {
            if let Some(slot) = self.slots.get_mut(preferred_slot as usize) {
                *slot = Some(JoystickSlot {
                    uuid,
                    name,
                    gilrs_id: id,
                    locked: true,
                    connected: true,
                });
                return;
            }
        }

        if let Some(empty_slot_idx) = self.find_empty_slot() {
            self.slots[empty_slot_idx] = Some(JoystickSlot {
                uuid,
                name: name.clone(),
                gilrs_id: id,
                locked: false,
                connected: true,
            });
        }
    }

    fn on_device_disconnected(&mut self, id: GamepadId) {
        for slot in &mut self.slots {
            if let Some(js) = slot.as_mut() {
                if js.gilrs_id == id {
                    if js.locked {
                        js.connected = false;
                    } else {
                        *slot = None;
                    }
                    return;
                }
            }
        }
    }

    fn read_gamepad(&self, id: GamepadId) -> JoystickData {
        let gamepad = self.gilrs.gamepad(id);

        let mut axes = Vec::with_capacity(6);

        axes.push(self.read_axis_value(&gamepad, gilrs::Axis::LeftStickX));

        axes.push(self.read_axis_value(&gamepad, gilrs::Axis::LeftStickY));

        let lt = self.read_button_value(&gamepad, gilrs::Button::LeftTrigger2);
        axes.push(lt);

        let rt = self.read_button_value(&gamepad, gilrs::Button::RightTrigger2);
        axes.push(rt);

        axes.push(self.read_axis_value(&gamepad, gilrs::Axis::RightStickX));

        axes.push(self.read_axis_value(&gamepad, gilrs::Axis::RightStickY));

        let mut buttons = Vec::with_capacity(10);
        for button_enum in [
            gilrs::Button::South,
            gilrs::Button::East,
            gilrs::Button::West,
            gilrs::Button::North,
            gilrs::Button::LeftTrigger,
            gilrs::Button::RightTrigger,
            gilrs::Button::Select,
            gilrs::Button::Start,
            gilrs::Button::LeftThumb,
            gilrs::Button::RightThumb,
        ] {
            buttons.push(gamepad.is_pressed(button_enum));
        }

        // Read D-pad as POV
        let pov = mapping::read_dpad_pov(&gamepad);
        let povs = vec![pov];

        JoystickData {
            axes,
            buttons,
            povs,
        }
    }

    fn find_empty_slot(&self) -> Option<usize> {
        self.slots.iter().position(|s| s.is_none())
    }

    fn uuid_for_gamepad(&self, id: GamepadId) -> String {
        let gamepad = self.gilrs.gamepad(id);
        format!("{:?}:{}", id, gamepad.name())
    }

    fn read_axis_value(&self, gamepad: &gilrs::Gamepad, axis: gilrs::Axis) -> i8 {
        if let Some(data) = gamepad.axis_data(axis) {
            (data.value() * 127.0).clamp(-128.0, 127.0) as i8
        } else {
            0
        }
    }

    fn read_button_value(&self, gamepad: &gilrs::Gamepad, button: gilrs::Button) -> i8 {
        if let Some(data) = gamepad.button_data(button) {
            (data.value() * 127.0).clamp(-128.0, 127.0) as i8
        } else {
            0
        }
    }
}
