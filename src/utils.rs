macro_rules! bit {
    ($bit_num:expr) => {
        (1 << $bit_num) as u8
    };
}

// Multiple bit selection.
macro_rules! bits {
    [ $($bit_num:expr),* ] => {
        $(bit!($bit_num))|*
    };
}

macro_rules! test_bit {
    ($val:expr, $bit_num:expr) => {
        ($val & bit!($bit_num)) != 0
    };
}

macro_rules! make_16 {
    ($hi:expr, $lo:expr) => {
        (($hi as u16) << 8) | ($lo as u16)
    };
}

macro_rules! hi_16 {
    ($val:expr) => {
        ($val >> 8) as u8
    };
}

macro_rules! lo_16 {
    ($val:expr) => {
        $val as u8
    };
}