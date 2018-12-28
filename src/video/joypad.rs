enum Select {
    Direction,
    Button,
    None
}

const SELECT_DIRECTION: u8  = 1 << 4;
const SELECT_BUTTONS: u8    = 1 << 5;

pub struct Joypad {
    pub a:      bool,
    pub b:      bool,
    pub select: bool,
    pub start:  bool,
    pub right:  bool,
    pub left:   bool,
    pub up:     bool,
    pub down:   bool,

    selector:   Select,
}

impl Joypad {
    pub fn new() -> Self {
        Joypad {
            a:      false,
            b:      false,
            select: false,
            start:  false,

            right:  false,
            left:   false,
            up:     false,
            down:   false,

            selector: Select::None
        }
    }

    pub fn read(&self) -> u8 {
        match self.selector {
            Select::Direction   => {
                let right = if self.right {1} else {0};
                let left =  if self.left {2} else {0};
                let up =    if self.up {4} else {0};
                let down =  if self.down {8} else {0};
                right | left | up | down
            },
            Select::Button      => {
                let a =         if self.a {1} else {0};
                let b =         if self.b {2} else {0};
                let select =    if self.select {4} else {0};
                let start =     if self.start {8} else {0};
                a | b | select | start
            },
            Select::None        => 0
        }
    }

    pub fn write(&mut self, val: u8) {
        if (val & SELECT_DIRECTION) != 0 {
            self.selector = Select::Direction;
        } else if (val & SELECT_BUTTONS) != 0 {
            self.selector = Select::Button;
        } else {
            self.selector = Select::None;
        }
    }
}
