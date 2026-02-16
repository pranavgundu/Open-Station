use gilrs::{EventType, GamepadId, Gilrs};
use open_station_protocol::types::JoystickData;
use std::collections::HashMap;

pub mod mapping;

/// Information about a joystick for the UI
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

/// A joystick mapped to an FRC slot
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
    slots: Vec<Option<JoystickSlot>>, // 6 slots
    locks: HashMap<String, u8>,       // UUID → preferred slot
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

    /// Poll for gamepad events (connect/disconnect). Call frequently.
    pub fn poll(&mut self) {
        while let Some(event) = self.gilrs.next_event() {
            match event.event {
                EventType::Connected => self.on_device_connected(event.id),
                EventType::Disconnected => self.on_device_disconnected(event.id),
                _ => {}
            }
        }
    }

    /// Get joystick data for all 6 slots (for sending to roboRIO)
    pub fn get_joystick_data(&self) -> Vec<JoystickData> {
        self.slots
            .iter()
            .map(|slot| match slot {
                Some(js) if js.connected => self.read_gamepad(js.gilrs_id),
                _ => JoystickData::default(),
            })
            .collect()
    }

    /// Get joystick info for the UI
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

    /// Reorder joysticks by UUID list
    pub fn reorder(&mut self, order: Vec<String>) {
        // Build new slot arrangement based on provided UUID order
        // Devices not in the list keep their current position
        let mut new_slots: Vec<Option<JoystickSlot>> = (0..6).map(|_| None).collect();

        // First pass: place devices from the order list
        for (target_slot, uuid) in order.iter().enumerate() {
            if target_slot >= 6 {
                break;
            }

            // Find the device with this UUID in current slots
            for current_slot in &mut self.slots {
                if let Some(js) = current_slot.as_ref() {
                    if &js.uuid == uuid {
                        // Move it to the new position
                        new_slots[target_slot] = current_slot.take();
                        break;
                    }
                }
            }
        }

        // Second pass: place remaining devices in empty slots
        let mut next_empty_slot = 0;
        for current_slot in &mut self.slots {
            if let Some(js) = current_slot.take() {
                // Find next empty slot
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

    /// Lock a joystick to a slot
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

    /// Unlock a joystick
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

    /// Force a full rescan
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

    /// Check if any joystick is connected
    pub fn any_connected(&self) -> bool {
        self.slots
            .iter()
            .any(|s| s.as_ref().is_some_and(|js| js.connected))
    }

    // Private helpers

    /// Scan all connected gamepads and assign them to slots
    fn scan_devices(&mut self) {
        let ids: Vec<GamepadId> = self.gilrs.gamepads().map(|(id, _)| id).collect();
        for id in ids {
            self.on_device_connected(id);
        }
    }

    /// Handle a new device connection
    fn on_device_connected(&mut self, id: GamepadId) {
        let gamepad = self.gilrs.gamepad(id);

        let uuid = self.uuid_for_gamepad(id);
        let name = gamepad.name().to_string();

        // Check if this device already exists in a slot
        for slot in &mut self.slots {
            if let Some(js) = slot.as_mut() {
                if js.uuid == uuid {
                    // Just mark it as connected
                    js.connected = true;
                    js.gilrs_id = id;
                    return;
                }
            }
        }

        // New device - check if it has a locked preferred slot
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

        // Find an empty slot
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

    /// Handle a device disconnection
    fn on_device_disconnected(&mut self, id: GamepadId) {
        for slot in &mut self.slots {
            if let Some(js) = slot.as_mut() {
                if js.gilrs_id == id {
                    if js.locked {
                        // Keep the slot but mark as disconnected
                        js.connected = false;
                    } else {
                        // Remove the slot entirely
                        *slot = None;
                    }
                    return;
                }
            }
        }
    }

    /// Read all input data from a gamepad
    fn read_gamepad(&self, id: GamepadId) -> JoystickData {
        let gamepad = self.gilrs.gamepad(id);

        // Read all 6 standard FRC axes
        let mut axes = Vec::with_capacity(6);

        // Axis 0: Left Stick X
        axes.push(self.read_axis_value(&gamepad, gilrs::Axis::LeftStickX));

        // Axis 1: Left Stick Y
        axes.push(self.read_axis_value(&gamepad, gilrs::Axis::LeftStickY));

        // Axis 2: Left Trigger (Triggers are often buttons with values in gilrs)
        // Use LeftTrigger2 (Analog). We avoid LeftTrigger because it maps to the bumper (L1).
        let lt = self.read_button_value(&gamepad, gilrs::Button::LeftTrigger2);
        axes.push(lt);

        // Axis 3: Right Trigger
        // Use RightTrigger2 (Analog). We avoid RightTrigger because it maps to the bumper (R1).
        let rt = self.read_button_value(&gamepad, gilrs::Button::RightTrigger2);
        axes.push(rt);

        // Axis 4: Right Stick X
        axes.push(self.read_axis_value(&gamepad, gilrs::Axis::RightStickX));

        // Axis 5: Right Stick Y
        axes.push(self.read_axis_value(&gamepad, gilrs::Axis::RightStickY));

        // Read all 10 standard FRC buttons
        let mut buttons = Vec::with_capacity(10);
        for button_enum in [
            gilrs::Button::South,        // A -> 1
            gilrs::Button::East,         // B -> 2
            gilrs::Button::West,         // X -> 3
            gilrs::Button::North,        // Y -> 4
            gilrs::Button::LeftTrigger,  // LB -> 5
            gilrs::Button::RightTrigger, // RB -> 6
            gilrs::Button::Select,       // Back -> 7
            gilrs::Button::Start,        // Start -> 8
            gilrs::Button::LeftThumb,    // LS -> 9
            gilrs::Button::RightThumb,   // RS -> 10
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

    /// Find the first empty slot
    fn find_empty_slot(&self) -> Option<usize> {
        self.slots.iter().position(|s| s.is_none())
    }

    /// Get a UUID string for a gamepad
    fn uuid_for_gamepad(&self, id: GamepadId) -> String {
        // Use the gamepad's unique identifier
        // gilrs doesn't provide a true UUID, so we construct one from the ID
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
