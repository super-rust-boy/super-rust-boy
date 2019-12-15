use bitflags::bitflags;

bitflags! {
    #[derive(Default)]
    pub struct Buttons: u8 {
        const START     = bit!(3);
        const SELECT    = bit!(2);
        const B         = bit!(1);
        const A         = bit!(0);
    }
}

bitflags! {
    #[derive(Default)]
    pub struct Directions: u8 {
        const DOWN  = bit!(3);
        const UP    = bit!(2);
        const LEFT  = bit!(1);
        const RIGHT = bit!(0);
    }
}

enum Select {
    Direction,
    Button,
    None
}

const SELECT_DIRECTION: u8  = bit!(4);
const SELECT_BUTTONS: u8    = bit!(5);

pub struct Joypad {
    buttons:    Buttons,
    directions: Directions,

    selector:   Select,
    change:     bool
}

impl Joypad {
    pub fn new() -> Self {
        Joypad {
            buttons:    Buttons::default(),
            directions: Directions::default(),

            selector:   Select::None,
            change:     false
        }
    }

    pub fn read(&self) -> u8 {
        match self.selector {
            Select::Direction => (!self.directions.bits() & 0xF),
            Select::Button => (!self.buttons.bits() & 0xF),
            Select::None => 0
        }
    }

    pub fn write(&mut self, val: u8) {
        self.selector = if (val & SELECT_BUTTONS) == 0 {
            Select::Button
        } else if (val & SELECT_DIRECTION) == 0 {
            Select::Direction
        } else {
            Select::None
        };
    }

    pub fn set_direction(&mut self, direction: Directions, val: bool) {
        self.directions.set(direction, val);
        self.change = self.change || val;
    }

    pub fn set_button(&mut self, button: Buttons, val: bool) {
        self.buttons.set(button, val);
        self.change = self.change || val;
    }

    pub fn check_interrupt(&mut self) -> bool {
        let trigger_interrupt = self.change;
        self.change = false;
        trigger_interrupt
    }
}
