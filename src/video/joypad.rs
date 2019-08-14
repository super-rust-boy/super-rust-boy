use bitflags::bitflags;

bitflags! {
    #[derive(Default)]
    pub struct Buttons: u8 {
        const START     = 0b00001000;
        const SELECT    = 0b00000100;
        const B         = 0b00000010;
        const A         = 0b00000001;
    }
}

bitflags! {
    #[derive(Default)]
    pub struct Directions: u8 {
        const DOWN  = 0b00001000;
        const UP    = 0b00000100;
        const LEFT  = 0b00000010;
        const RIGHT = 0b00000001;
    }
}

enum Select {
    Direction,
    Button,
    None
}

const SELECT_DIRECTION: u8  = 1 << 4;
const SELECT_BUTTONS: u8    = 1 << 5;

pub struct Joypad {
    pub buttons:    Buttons,
    pub directions: Directions,

    selector:       Select
}

impl Joypad {
    pub fn new() -> Self {
        Joypad {
            buttons:    Buttons::default(),
            directions: Directions::default(),

            selector:   Select::None
        }
    }

    pub fn read(&self) -> u8 {
        match self.selector {
            Select::Direction => !self.directions.bits(),
            Select::Button => !self.buttons.bits(),
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
}
